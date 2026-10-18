//! `CancellableResponseStream` -- a response stream whose reads end when a signal fires.

use std::marker::PhantomData;
use std::sync::Arc;

use grpc_egress_transport::CancellationSignal;
use proto_pattern::{CallError, MessageStream, ResponseStream, StatusCode};

/// A response stream of messages `R` that fails with `Cancelled` when `signal` fires while a message or the trailers
/// are being read.
pub(crate) struct CancellableResponseStream<S, R, G> {
    inner: S,
    signal: Arc<G>,
    path: Box<str>,
    message: PhantomData<fn() -> R>,
}

impl<S, R, G> CancellableResponseStream<S, R, G> {
    /// `inner`, ended by `signal`; `path` names the call in the failure.
    pub(crate) fn create(inner: S, signal: Arc<G>, path: &str) -> Self {
        Self {
            inner,
            signal,
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

impl<R: Send, S: ResponseStream<R>, G: CancellationSignal> MessageStream<R>
    for CancellableResponseStream<S, R, G>
{
    async fn read_message(&mut self) -> Result<Option<R>, CallError> {
        tokio::select! {
            biased;
            () = self.signal.wait_cancelled() => Err(self.cancelled()),
            message = self.inner.read_message() => message,
        }
    }
}

impl<R: Send, S: ResponseStream<R>, G: CancellationSignal> ResponseStream<R>
    for CancellableResponseStream<S, R, G>
{
    type Metadata = S::Metadata;

    fn get_headers(&self) -> &S::Metadata {
        self.inner.get_headers()
    }

    async fn read_trailers(&mut self) -> Result<S::Metadata, CallError> {
        tokio::select! {
            biased;
            () = self.signal.wait_cancelled() => Err(self.cancelled()),
            trailers = self.inner.read_trailers() => trailers,
        }
    }
}
