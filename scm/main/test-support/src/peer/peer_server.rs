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

use crate::{GrpcFrame, PeerBehaviour, RecordedRequest, RunningPeer};

/// Starts in-process peers that answer unary gRPC calls and record what they received.
pub struct PeerServer;

impl PeerServer {
    /// Start a peer on a port the operating system assigns, answering as `behaviour` says.
    ///
    /// # Errors
    ///
    /// The error of binding the listener or reading its address.
    pub async fn start(behaviour: PeerBehaviour) -> io::Result<RunningPeer> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let requests: Arc<Mutex<Vec<RecordedRequest>>> = Arc::new(Mutex::new(Vec::new()));

        let record = requests.clone();
        let accept_loop = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let behaviour = behaviour.clone();
                let record = record.clone();
                tokio::spawn(async move {
                    let service = service_fn(move |request: http::Request<Incoming>| {
                        Self::answer(request, behaviour.clone(), record.clone())
                    });
                    // A connection that ends with an error is the client's concern in a test.
                    let _ = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
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
        } = &behaviour_for_headers
        {
            builder = builder.header("grpc-encoding", encoding.as_str());
        }
        let response = builder
            .body(Full::new(reply))
            .unwrap_or_else(|_| http::Response::new(Full::new(Bytes::new())));
        Ok(response)
    }
}
