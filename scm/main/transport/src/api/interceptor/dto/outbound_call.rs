//! `OutboundCall` -- what a call is about to carry, for [`GrpcEgressInterceptor::before_call`](crate::GrpcEgressInterceptor::before_call).

use std::time::Duration;

use proto_pattern::Metadata;

/// A call that has not been sent: its path, its timeout and the request headers it will carry. An
/// interceptor changes `metadata` (and may change `timeout`) and the call is sent with the result.
#[derive(Debug, Clone)]
pub struct OutboundCall<'a> {
    /// The path of the method, `/package.Service/Method`.
    pub path: &'a str,
    /// The timeout the call carries, when it has one.
    pub timeout: Option<Duration>,
    /// The request headers the call will carry.
    pub metadata: Metadata,
}
