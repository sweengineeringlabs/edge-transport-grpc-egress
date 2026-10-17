//! `CancellableResponseStream` -- a response stream whose reads end when a token fires.

use std::marker::PhantomData;

use proto_pattern::{CallError, MessageStream, ResponseStream, StatusCode};
use tokio_util::sync::CancellationToken;

/// A response stream of messages `R` that fails with `Cancelled` when `token` fires while a message or the
/// trailers are being read.
pub(crate) struct CancellableResponseStream<S, R> {
    inner: S,
    token: CancellationToken,
    path: Box<str>,
    message: PhantomData<fn() -> R>,
}

impl<S, R> CancellableResponseStream<S, R> {
    /// `inner`, ended by `token`; `path` names the call in the failure.
    pub(crate) fn create(inner: S, token: CancellationToken, path: &str) -> Self {
        Self {
            inner,
            token,
            path: path.into(),
            message: PhantomData,
        }
    }

    fn cancelled(&self) -> CallError {
        CallError::Transport {
            path: self.path.clone(),
            code: StatusCode::Cancelled,
            reason: "the caller cancelled the call".to_string(),
        }
    }
}

impl<R: Send, S: ResponseStream<R>> MessageStream<R> for CancellableResponseStream<S, R> {
    async fn read_message(&mut self) -> Result<Option<R>, CallError> {
        tokio::select! {
            biased;
            () = self.token.cancelled() => Err(self.cancelled()),
            message = self.inner.read_message() => message,
        }
    }
}

impl<R: Send, S: ResponseStream<R>> ResponseStream<R> for CancellableResponseStream<S, R> {
    type Metadata = S::Metadata;

    fn get_headers(&self) -> &S::Metadata {
        self.inner.get_headers()
    }

    async fn read_trailers(&mut self) -> Result<S::Metadata, CallError> {
        tokio::select! {
            biased;
            () = self.token.cancelled() => Err(self.cancelled()),
            trailers = self.inner.read_trailers() => trailers,
        }
    }
}
