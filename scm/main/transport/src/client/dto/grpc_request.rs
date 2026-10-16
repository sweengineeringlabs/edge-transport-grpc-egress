//! gRPC request envelope.

use std::collections::HashMap;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

/// A gRPC request envelope.
///
/// ## Mandatory deadline
///
/// Every gRPC call MUST carry a per-call deadline.  Production gRPC
/// without deadlines deadlocks on slow servers or stuck connections —
/// the deadline is therefore a *required* positional argument of
/// [`GrpcRequest::new`], not an `Option`.  Construction without a
/// `Duration` is a compile error.
///
/// ## Optional cancellation
///
/// A caller-supplied [`CancellationToken`] can be attached via
/// [`GrpcRequest::with_cancellation`].  When the token fires, the
/// in-flight request is aborted by the egress transport.  Default:
/// no token (caller never cancels).
#[derive(Debug, Clone)]
pub struct GrpcRequest {
    /// Fully-qualified gRPC method path (e.g. `"pkg.Service/Method"`).
    pub method: String,
    /// Raw protobuf-encoded request bytes.
    pub body: Vec<u8>,
    /// Request metadata (headers).
    pub metadata: HashMap<String, String>,
    /// Per-call deadline.  Always present.
    pub deadline: Duration,
    /// Optional caller-supplied cancellation token.
    pub cancellation: Option<CancellationToken>,
}
