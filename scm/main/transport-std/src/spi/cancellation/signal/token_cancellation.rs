//! `TokenCancellation` -- a `CancellationToken` as a [`CancellationSignal`].

use grpc_egress_transport::CancellationSignal;
use tokio_util::sync::CancellationToken;

/// A [`CancellationToken`] of `tokio-util`, offered as the interface the cancellation view knows. The caller keeps the
/// token (it is a handle: clone it, and cancel the clone).
pub struct TokenCancellation {
    token: CancellationToken,
}

impl From<CancellationToken> for TokenCancellation {
    /// The signal that fires when `token` is cancelled.
    fn from(token: CancellationToken) -> Self {
        Self { token }
    }
}

impl CancellationSignal for TokenCancellation {
    async fn wait_cancelled(&self) {
        self.token.cancelled().await;
    }
}
