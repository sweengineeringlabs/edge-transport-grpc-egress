//! `CancellableTransport` -- a view of a client whose calls end when a token fires.

use std::future::Future;

use proto_pattern::{
    BidiStreamMethod, CallError, CallOptions, CallRequest, CallResponse, ClientStreamMethod,
    MessageStream, MethodPath, RawTransport, ResponseStream, ServerStreamMethod, StatusCode,
    Transport, UnaryMethod,
};
use tokio_util::sync::CancellationToken;

use super::CancellableResponseStream;

/// A view of `inner` bound to a token: a call in flight, and every message read from a stream it opened, fails with
/// `Cancelled` when the token fires. Dropping the future of a call cancels the call without it; this is for a
/// caller that holds a token. The view borrows the client and holds a token handle, so making one costs nothing
/// and a client used without one pays nothing.
pub(crate) struct CancellableTransport<'a, T> {
    inner: &'a T,
    token: CancellationToken,
}

impl<'a, T> CancellableTransport<'a, T> {
    /// A view of `inner` ended by `token`.
    pub(crate) fn create(inner: &'a T, token: CancellationToken) -> Self {
        Self { inner, token }
    }

    fn cancelled(path: &str) -> CallError {
        CallError::Transport {
            path: path.into(),
            code: StatusCode::Cancelled,
            reason: "the caller cancelled the call".to_string(),
        }
    }

    /// `call`, or the failure when the token fires first.
    async fn race<R>(
        &self,
        path: &str,
        call: impl Future<Output = Result<R, CallError>>,
    ) -> Result<R, CallError> {
        tokio::select! {
            biased;
            () = self.token.cancelled() => Err(Self::cancelled(path)),
            result = call => result,
        }
    }
}

impl<T: Transport + RawTransport> Transport for CancellableTransport<'_, T> {
    type Metadata = <T as Transport>::Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        self.race(M::PATH, self.inner.call::<M>(request)).await
    }

    async fn call_server_stream<M: ServerStreamMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>, CallError>
    {
        let stream = self
            .race(M::PATH, self.inner.call_server_stream::<M>(request))
            .await?;
        Ok(CancellableResponseStream::create(
            stream,
            self.token.clone(),
            M::PATH,
        ))
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        requests: S,
        options: CallOptions,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        self.race(
            M::PATH,
            self.inner.call_client_stream::<M, S>(requests, options),
        )
        .await
    }

    async fn call_bidi_stream<M: BidiStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        requests: S,
        options: CallOptions,
    ) -> Result<impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>, CallError>
    {
        let stream = self
            .race(
                M::PATH,
                self.inner.call_bidi_stream::<M, S>(requests, options),
            )
            .await?;
        Ok(CancellableResponseStream::create(
            stream,
            self.token.clone(),
            M::PATH,
        ))
    }
}

impl<T: Transport + RawTransport> RawTransport for CancellableTransport<'_, T> {
    type Metadata = <T as RawTransport>::Metadata;

    async fn call_raw(
        &self,
        path: MethodPath,
        request: CallRequest<Vec<u8>>,
    ) -> Result<CallResponse<Vec<u8>, <Self as RawTransport>::Metadata>, CallError> {
        let text = path.get_path().to_string();
        self.race(&text, self.inner.call_raw(path, request)).await
    }

    async fn call_raw_server_stream(
        &self,
        path: MethodPath,
        request: CallRequest<Vec<u8>>,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>, CallError>
    {
        let text = path.get_path().to_string();
        let stream = self
            .race(&text, self.inner.call_raw_server_stream(path, request))
            .await?;
        Ok(CancellableResponseStream::create(
            stream,
            self.token.clone(),
            &text,
        ))
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        requests: S,
        options: CallOptions,
    ) -> Result<CallResponse<Vec<u8>, <Self as RawTransport>::Metadata>, CallError> {
        let text = path.get_path().to_string();
        self.race(
            &text,
            self.inner.call_raw_client_stream(path, requests, options),
        )
        .await
    }

    async fn call_raw_bidi_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        requests: S,
        options: CallOptions,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>, CallError>
    {
        let text = path.get_path().to_string();
        let stream = self
            .race(
                &text,
                self.inner.call_raw_bidi_stream(path, requests, options),
            )
            .await?;
        Ok(CancellableResponseStream::create(
            stream,
            self.token.clone(),
            &text,
        ))
    }
}
