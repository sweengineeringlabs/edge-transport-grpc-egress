//! [`PeerServer`] -- an in-process gRPC peer over HTTP/2.

use std::collections::HashMap;
use std::convert::Infallible;
use std::io;
use std::sync::{Arc, Mutex, PoisonError};

use bytes::{Bytes, BytesMut};
use http_body_util::{BodyExt as _, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::{
    Fixture, GrpcFrame, PeerBehaviour, RecordedRequest, RunningPeer, TestPki, TlsServerMode,
};

/// Starts in-process peers that answer unary gRPC calls and record what they received.
pub struct PeerServer;

impl PeerServer {
    /// Start a peer on a port the operating system assigns, answering as `behaviour` says.
    ///
    /// # Errors
    ///
    /// The error of binding the listener or reading its address.
    pub async fn start(behaviour: PeerBehaviour) -> io::Result<RunningPeer> {
        Self::serve(behaviour, None).await
    }

    /// Start a peer that speaks TLS with the server certificate of `pki`, demanding what `mode`
    /// says of the client. A connection whose handshake fails is dropped.
    ///
    /// # Errors
    ///
    /// The error of building the TLS configuration, binding the listener or reading its address.
    pub async fn start_tls(
        behaviour: PeerBehaviour,
        pki: &TestPki,
        mode: TlsServerMode,
    ) -> io::Result<RunningPeer> {
        // With two crypto backends in one build (a workspace), rustls cannot pick one by itself.
        Fixture::ensure_tls_provider();
        let builder = rustls::ServerConfig::builder();
        let builder = match mode {
            TlsServerMode::ServerAuthOnly => builder.with_no_client_auth(),
            TlsServerMode::ClientAuthRequired => {
                let mut roots = rustls::RootCertStore::empty();
                for root in pki.get_client_roots() {
                    roots.add(root).map_err(io::Error::other)?;
                }
                let verifier = rustls::server::WebPkiClientVerifier::builder(roots.into())
                    .build()
                    .map_err(io::Error::other)?;
                builder.with_client_cert_verifier(verifier)
            }
        };
        let mut config = builder
            .with_single_cert(pki.get_server_chain(), pki.get_server_key())
            .map_err(io::Error::other)?;
        config.alpn_protocols = vec![b"h2".to_vec()];
        Self::serve(
            behaviour,
            Some(TlsAcceptor::from(std::sync::Arc::new(config))),
        )
        .await
    }

    async fn serve(
        behaviour: PeerBehaviour,
        acceptor: Option<TlsAcceptor>,
    ) -> io::Result<RunningPeer> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let requests: Arc<Mutex<Vec<RecordedRequest>>> = Arc::new(Mutex::new(Vec::new()));

        let record = requests.clone();
        let accept_loop = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let behaviour = behaviour.clone();
                let record = record.clone();
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let service = service_fn(move |request: http::Request<Incoming>| {
                        Self::answer(request, behaviour.clone(), record.clone())
                    });
                    let server = hyper::server::conn::http2::Builder::new(TokioExecutor::new());
                    // A connection that ends with an error is the client's concern in a test.
                    match acceptor {
                        None => {
                            let _ = server.serve_connection(TokioIo::new(stream), service).await;
                        }
                        Some(acceptor) => {
                            if let Ok(tls) = acceptor.accept(stream).await {
                                let _ = server.serve_connection(TokioIo::new(tls), service).await;
                            }
                        }
                    }
                });
            }
        });
        Ok(RunningPeer::create(addr, requests, accept_loop))
    }

    async fn answer(
        request: http::Request<Incoming>,
        behaviour: PeerBehaviour,
        record: Arc<Mutex<Vec<RecordedRequest>>>,
    ) -> Result<http::Response<Full<Bytes>>, Infallible> {
        let path = request.uri().path().to_string();
        let headers: HashMap<String, String> = request
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|v| (name.as_str().to_string(), v.to_string()))
            })
            .collect();
        let body = match request.into_body().collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_) => Bytes::new(),
        };
        let frames = GrpcFrame::decode_all(&body);

        let reply = match &behaviour {
            PeerBehaviour::Reply(payload) => GrpcFrame::encode(payload),
            PeerBehaviour::ReplyCompressed { payload, .. } => GrpcFrame::encode_compressed(payload),
            PeerBehaviour::ReplyRaw { body, .. } => Bytes::from(body.clone()),
            PeerBehaviour::ReplyStatus { .. } => Bytes::new(),
            PeerBehaviour::Echo | PeerBehaviour::Delay(_) => {
                let mut echoed = BytesMut::new();
                for frame in &frames {
                    echoed.extend_from_slice(&GrpcFrame::encode(frame.get_payload()));
                }
                echoed.freeze()
            }
        };
        record
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(RecordedRequest::create(path, headers, frames));

        let behaviour_for_headers = behaviour.clone();
        if let PeerBehaviour::Delay(wait) = behaviour {
            tokio::time::sleep(wait).await;
        }

        let mut builder = http::Response::builder()
            .status(200)
            .header(http::header::CONTENT_TYPE, "application/grpc")
            .header("grpc-status", "0");
        if let PeerBehaviour::ReplyCompressed {
            encoding: Some(encoding),
            ..
        }
        | PeerBehaviour::ReplyRaw {
            encoding: Some(encoding),
            ..
        } = &behaviour_for_headers
        {
            builder = builder.header("grpc-encoding", encoding.as_str());
        }
        if let PeerBehaviour::ReplyStatus {
            status,
            message,
            retry_after_secs,
        } = &behaviour_for_headers
        {
            if let Some(headers) = builder.headers_mut() {
                headers.insert(
                    "grpc-status",
                    status
                        .parse()
                        .unwrap_or_else(|_| http::HeaderValue::from_static("")),
                );
            }
            if let Some(message) = message {
                builder = builder.header("grpc-message", message.as_str());
            }
            if let Some(secs) = retry_after_secs {
                builder = builder.header("retry-after", secs.to_string());
            }
        }
        let response = builder
            .body(Full::new(reply))
            .unwrap_or_else(|_| http::Response::new(Full::new(Bytes::new())));
        Ok(response)
    }
}
