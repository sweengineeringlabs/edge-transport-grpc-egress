//! `InboundReply` -- what a call got back, for [`GrpcEgressInterceptor::after_call`](crate::GrpcEgressInterceptor::after_call).

use proto_pattern::Metadata;

/// The response of a call that came back with a message: its path and the headers and trailers the peer sent. The
/// message itself is not here: the layer that hands a reply to an interceptor does not know its type.
#[derive(Debug, Clone)]
pub struct InboundReply<'a> {
    /// The path of the method, `/package.Service/Method`.
    pub path: &'a str,
    /// The response headers.
    pub headers: Metadata,
    /// The response trailers.
    pub trailers: Metadata,
}
