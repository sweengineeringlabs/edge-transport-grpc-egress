//! `ProtoSvcGrpcEgress` -- the `GrpcEgress` port over a `proto-svc` [`RawTransport`].

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use proto_pattern::{CallOptions, CallRequest, MessageStream, MethodPath, RawTransport};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use super::payload_source::{PayloadSource, StreamFailure};
use super::EgressCallMapper;
use crate::std::GrpcEgressInterceptorChain;
use grpc_egress_transport::{
    CallStreamRequest, GrpcEgress, GrpcEgressError, GrpcEgressResult, GrpcMessageStreamResponse,
    GrpcRequest, GrpcResponse, HealthCheckRequest,
};

/// Messages a response stream holds before the pump waits for its reader.
const RESPONSE_STREAM_BUFFER: usize = 16;

/// The sender that reports whether a response stream opened.
type OpenedSender = oneshot::Sender<GrpcEgressResult<()>>;

/// The sender that carries the messages of a response stream to its reader.
type MessageSender = mpsc::Sender<GrpcEgressResult<Vec<u8>>>;

/// What a caller is told when the reason is internal: the reason itself goes to the log.
const SANITIZED_INTERNAL_MSG: &str = "internal client error";

/// A [`GrpcEgress`] that makes its calls through `T`, the transport `proto-svc` built for one endpoint.
///
/// Interceptors run before and after exactly as they did on the client this one replaces, and the
/// request timeout is a backstop: a unary call ends at the earlier of its own deadline and the timeout.
pub(crate) struct ProtoSvcGrpcEgress<T> {
    transport: Arc<T>,
    base_uri: String,
    timeout: Duration,
    interceptors: GrpcEgressInterceptorChain,
}

impl<T> ProtoSvcGrpcEgress<T> {
    /// A client for `base_uri` that calls through `transport`.
    pub(crate) fn create(
        transport: T,
        base_uri: impl Into<String>,
        timeout: Duration,
        interceptors: GrpcEgressInterceptorChain,
    ) -> Self {
        Self {
            transport: Arc::new(transport),
            base_uri: base_uri.into(),
            timeout,
            interceptors,
        }
    }

    /// Race `call` against the caller's cancellation token.
    ///
    /// # Errors
    ///
    /// `Cancelled` when the token fires first.
    async fn race_cancel<F: Future>(
        call: F,
        cancel: Option<&CancellationToken>,
    ) -> GrpcEgressResult<F::Output> {
        match cancel {
            Some(token) => tokio::select! {
                biased;
                _ = token.cancelled() => Err(GrpcEgressError::Cancelled(
                    "caller cancelled in-flight request".into(),
                )),
                output = call => Ok(output),
            },
            None => Ok(call.await),
        }
    }

    /// The options of a call: its `deadline` and the `metadata` of the caller.
    fn create_options(
        metadata: &std::collections::HashMap<String, String>,
        deadline: Duration,
    ) -> GrpcEgressResult<CallOptions> {
        Ok(CallOptions::create(
            Some(deadline),
            EgressCallMapper::create_metadata(metadata)?,
        ))
    }
}

impl<T: RawTransport + Send + Sync + 'static> ProtoSvcGrpcEgress<T> {
    /// Run `task`, which opens a response stream and pumps it, and hand the stream to the caller as a
    /// `'static` one.
    ///
    /// The contract's streams borrow the transport, and [`GrpcMessageStreamResponse`] cannot, so a task
    /// that owns a share of the transport opens the stream and pumps its messages into a channel. The
    /// task reports the result of opening before this returns, so a call that could not start fails here
    /// and not at the first message; dropping the returned stream ends the task.
    async fn pump_responses<F, Fut>(&self, task: F) -> GrpcEgressResult<GrpcMessageStreamResponse>
    where
        F: FnOnce(Arc<T>, OpenedSender, MessageSender) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let (opened_tx, opened_rx) = oneshot::channel::<GrpcEgressResult<()>>();
        let (message_tx, message_rx) = mpsc::channel(RESPONSE_STREAM_BUFFER);
        tokio::spawn(task(Arc::clone(&self.transport), opened_tx, message_tx));
        opened_rx.await.map_err(|_| {
            tracing::warn!("the task that opens a gRPC stream ended before it reported");
            GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
        })??;
        Ok(GrpcMessageStreamResponse {
            stream: Box::pin(futures::stream::unfold(message_rx, |mut rx| async move {
                rx.recv().await.map(|item| (item, rx))
            })),
        })
    }

    /// Report the result of opening a stream and, when it opened, pump its messages to `message_tx` until
    /// the stream ends, fails, or its reader is gone.
    async fn drain_opened<S: MessageStream<Vec<u8>>>(
        opened: Result<S, proto_pattern::CallError>,
        opened_tx: OpenedSender,
        message_tx: MessageSender,
    ) {
        let mut stream = match opened {
            Ok(stream) => {
                let _ = opened_tx.send(Ok(()));
                stream
            }
            Err(error) => {
                let _ = opened_tx.send(Err(EgressCallMapper::adapt_error(error)));
                return;
            }
        };
        loop {
            let item = match stream.read_message().await {
                Ok(Some(message)) => Ok(message),
                Ok(None) => return,
                Err(error) => Err(EgressCallMapper::adapt_error(error)),
            };
            let failed = item.is_err();
            if message_tx.send(item).await.is_err() || failed {
                return;
            }
        }
    }

    /// The single response of a call, as this crate's type, after the after-call interceptors.
    fn finish_response(
        &self,
        response: proto_pattern::CallResponse<Vec<u8>, impl proto_pattern::MetadataView>,
    ) -> GrpcEgressResult<GrpcResponse> {
        let metadata = EgressCallMapper::read_metadata(response.get_trailers());
        let mut response = GrpcResponse {
            body: response.into_message(),
            metadata,
        };
        self.interceptors.run_after(&mut response)?;
        Ok(response)
    }

    /// The path, options and request of a stream call that carries `metadata` and runs to `deadline`.
    fn create_call(
        method: &str,
        body: Vec<u8>,
        metadata: &std::collections::HashMap<String, String>,
        deadline: Duration,
    ) -> GrpcEgressResult<(MethodPath, CallRequest<Vec<u8>>)> {
        Ok((
            EgressCallMapper::create_path(method)?,
            CallRequest::create(body, Self::create_options(metadata, deadline)?),
        ))
    }

    /// The error a broken request stream ended its call with, else `error`.
    fn pick_error(failure: &StreamFailure, error: GrpcEgressError) -> GrpcEgressError {
        failure
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
            .unwrap_or(error)
    }
}

impl<T: RawTransport + Send + Sync + 'static> GrpcEgress for ProtoSvcGrpcEgress<T> {
    fn call_unary(
        &self,
        mut request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async move {
            self.interceptors.run_before(&mut request)?;
            // The client's request timeout is a backstop: a call never outlasts it.
            let deadline = request.deadline.min(self.timeout);
            let (path, call) =
                Self::create_call(&request.method, request.body, &request.metadata, deadline)?;
            let response = Self::race_cancel(
                self.transport.call_raw(path, call),
                request.cancellation.as_ref(),
            )
            .await?
            .map_err(EgressCallMapper::adapt_error)?;
            self.finish_response(response)
        })
    }

    fn call_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        self.call_bidi_stream(req)
    }

    fn call_server_stream(
        &self,
        mut request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        Box::pin(async move {
            self.interceptors.run_before(&mut request)?;
            let (path, call) = Self::create_call(
                &request.method,
                request.body,
                &request.metadata,
                request.deadline,
            )?;
            let opened = self.pump_responses(move |transport, opened_tx, message_tx| async move {
                let opened = transport.call_raw_server_stream(path, call).await;
                Self::drain_opened(opened, opened_tx, message_tx).await;
            });
            Self::race_cancel(opened, request.cancellation.as_ref()).await?
        })
    }

    fn call_client_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async move {
            let path = EgressCallMapper::create_path(&req.method)?;
            let options = Self::create_options(&req.metadata, self.timeout)?;
            let (source, failure) = PayloadSource::create(req.messages.stream);
            let response = self
                .transport
                .call_raw_client_stream(path, source, options)
                .await
                .map_err(|error| {
                    Self::pick_error(&failure, EgressCallMapper::adapt_error(error))
                })?;
            self.finish_response(response)
        })
    }

    fn call_bidi_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        Box::pin(async move {
            let path = EgressCallMapper::create_path(&req.method)?;
            let options = Self::create_options(&req.metadata, self.timeout)?;
            let (source, failure) = PayloadSource::create(req.messages.stream);
            self.pump_responses(move |transport, opened_tx, message_tx| async move {
                let opened = transport.call_raw_bidi_stream(path, source, options).await;
                Self::drain_opened(opened, opened_tx, message_tx).await;
            })
            .await
            .map_err(|error| Self::pick_error(&failure, error))
        })
    }

    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        let base_uri = self.base_uri.clone();
        Box::pin(async move {
            let uri: http::Uri = base_uri.parse().map_err(|e| {
                tracing::warn!(error = %e, uri = %base_uri, "invalid gRPC URI in health check");
                GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
            })?;
            let host = uri.host().unwrap_or("127.0.0.1").to_owned();
            let port = uri.port_u16().unwrap_or(50051);
            let addr = format!("{host}:{port}");
            tokio::net::TcpStream::connect(&addr)
                .await
                .map(|_| ())
                .map_err(|e| GrpcEgressError::Unavailable(format!("{addr}: {e}")))
        })
    }
}
