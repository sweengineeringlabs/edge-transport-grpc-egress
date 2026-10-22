//! `CancellableTransport` -- a view of a client whose calls end when a signal fires.

use proto_pattern::{
    BidiStreamMethod, CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallRawBidiStreamRequest, CallRawBidiStreamRequestBuilder,
    CallRawBidiStreamResponse, CallRawClientStreamRequest, CallRawClientStreamRequestBuilder,
    CallRawClientStreamResponse, CallRawRequest, CallRawRequestBuilder, CallRawResponse,
    CallRawServerStreamRequest, CallRawServerStreamRequestBuilder, CallRawServerStreamResponse,
    CallRequest, CallResponse, CallServerStreamRequest, CallServerStreamResponse,
    ClientStreamMethod, MessageStream, RawTransport, ResponseStream, ServerStreamMethod,
    StatusCode, Transport, UnaryMethod,
};
use std::future::Future;
use std::sync::Arc;

use grpc_egress_transport::CancellationSignal;

use super::CancellableResponseStream;

/// A view of `inner` bound to a signal: a call in flight, and every message read from a stream it opened, fails with
/// `Cancelled` when the signal fires. Dropping the future of a call cancels the call without it; this is for a
/// caller that holds a signal. The view borrows the client and shares the signal, so making one costs nothing and a
/// client used without one pays nothing.
pub(crate) struct CancellableTransport<'a, T, S> {
    inner: &'a T,
    signal: Arc<S>,
}

impl<'a, T, S: CancellationSignal> CancellableTransport<'a, T, S> {
    /// A view of `inner` ended by `signal`.
    pub(crate) fn create(inner: &'a T, signal: S) -> Self {
        Self {
            inner,
            signal: Arc::new(signal),
        }
    }

    fn cancelled(path: &str) -> CallError {
        CallError::Transport {
            path: path.into(),
            code: StatusCode::Cancelled,
            reason: "the caller cancelled the call".to_string(),
        }
    }

    /// `call`, or the failure when the signal fires first.
    async fn race<R>(
        &self,
        path: &str,
        call: impl Future<Output = Result<R, CallError>>,
    ) -> Result<R, CallError> {
        tokio::select! {
            biased;
            () = self.signal.wait_cancelled() => Err(Self::cancelled(path)),
            result = call => result,
        }
    }
}

impl<T: Transport + RawTransport, S: CancellationSignal + 'static> Transport
    for CancellableTransport<'_, T, S>
{
    type Metadata = <T as Transport>::Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        self.race(M::PATH, self.inner.call::<M>(request)).await
    }

    async fn call_server_stream<M: ServerStreamMethod>(
        &self,
        request: CallServerStreamRequest<M::Request>,
    ) -> Result<
        CallServerStreamResponse<
            impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>,
        >,
        CallError,
    > {
        let stream = self
            .race(M::PATH, async {
                self.inner
                    .call_server_stream::<M>(request)
                    .await
                    .map(CallServerStreamResponse::into_stream)
            })
            .await?;
        Ok(CallServerStreamResponse::new(
            CancellableResponseStream::create(stream, Arc::clone(&self.signal), M::PATH),
        ))
    }

    async fn call_client_stream<M: ClientStreamMethod, R: MessageStream<M::Request> + 'static>(
        &self,
        request: CallClientStreamRequest<R>,
    ) -> Result<CallClientStreamResponse<M::Response, <Self as Transport>::Metadata>, CallError>
    {
        self.race(M::PATH, self.inner.call_client_stream::<M, R>(request))
            .await
    }

    async fn call_bidi_stream<M: BidiStreamMethod, R: MessageStream<M::Request> + 'static>(
        &self,
        request: CallBidiStreamRequest<R>,
    ) -> Result<
        CallBidiStreamResponse<
            impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>,
        >,
        CallError,
    > {
        let stream = self
            .race(M::PATH, async {
                self.inner
                    .call_bidi_stream::<M, R>(request)
                    .await
                    .map(CallBidiStreamResponse::into_stream)
            })
            .await?;
        Ok(CallBidiStreamResponse::new(
            CancellableResponseStream::create(stream, Arc::clone(&self.signal), M::PATH),
        ))
    }
}

impl<T: Transport + RawTransport, S: CancellationSignal + 'static> RawTransport
    for CancellableTransport<'_, T, S>
{
    type Metadata = <T as RawTransport>::Metadata;

    async fn call_raw(
        &self,
        request: CallRawRequest,
    ) -> Result<CallRawResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, message, options) = request.into_parts();
        let text = path.get_path().to_string();
        self.race(
            &text,
            self.inner.call_raw(
                CallRawRequestBuilder::new()
                    .with_path(path)
                    .with_message(message)
                    .with_options(options)
                    .build(),
            ),
        )
        .await
    }

    async fn call_raw_server_stream(
        &self,
        request: CallRawServerStreamRequest,
    ) -> Result<
        CallRawServerStreamResponse<
            impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>,
        >,
        CallError,
    > {
        let (path, message, options) = request.into_parts();
        let text = path.get_path().to_string();
        let stream = self
            .race(&text, async {
                self.inner
                    .call_raw_server_stream(
                        CallRawServerStreamRequestBuilder::new()
                            .with_path(path)
                            .with_message(message)
                            .with_options(options)
                            .build(),
                    )
                    .await
                    .map(CallRawServerStreamResponse::into_stream)
            })
            .await?;
        Ok(CallRawServerStreamResponse::new(
            CancellableResponseStream::create(stream, Arc::clone(&self.signal), &text),
        ))
    }

    async fn call_raw_client_stream<R: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawClientStreamRequest<R>,
    ) -> Result<CallRawClientStreamResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, requests, options) = request.into_parts();
        let text = path.get_path().to_string();
        self.race(
            &text,
            self.inner.call_raw_client_stream(
                CallRawClientStreamRequestBuilder::new()
                    .with_path(path)
                    .with_requests(requests)
                    .with_options(options)
                    .build(),
            ),
        )
        .await
    }

    async fn call_raw_bidi_stream<R: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawBidiStreamRequest<R>,
    ) -> Result<
        CallRawBidiStreamResponse<
            impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>,
        >,
        CallError,
    > {
        let (path, requests, options) = request.into_parts();
        let text = path.get_path().to_string();
        let stream = self
            .race(&text, async {
                self.inner
                    .call_raw_bidi_stream(
                        CallRawBidiStreamRequestBuilder::new()
                            .with_path(path)
                            .with_requests(requests)
                            .with_options(options)
                            .build(),
                    )
                    .await
                    .map(CallRawBidiStreamResponse::into_stream)
            })
            .await?;
        Ok(CallRawBidiStreamResponse::new(
            CancellableResponseStream::create(stream, Arc::clone(&self.signal), &text),
        ))
    }
}
