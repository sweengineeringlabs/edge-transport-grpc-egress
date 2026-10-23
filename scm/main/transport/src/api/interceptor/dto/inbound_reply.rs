//! `InboundReply` -- what a call got back, for [`GrpcEgressInterceptor::after_call`](crate::GrpcEgressInterceptor::after_call).

use proto_pattern::Metadata;

/// The response of a call that came back with a message: its path and the headers and trailers the peer sent. The
/// message itself is not here: the layer that hands a reply to an interceptor does not know its type.
///
/// The fields are private so callbacks cannot mutate the response metadata through this view.
///
/// ```compile_fail
/// use grpc_egress_transport::{InboundReply, proto_pattern::Metadata};
/// let _ = InboundReply { path: "/svc/M", headers: Metadata::default(), trailers: Metadata::default() };
/// ```
#[derive(Debug, Clone)]
pub struct InboundReply<'a> {
    /// The path of the method, `/package.Service/Method`.
    path: &'a str,
    /// The response headers.
    headers: Metadata,
    /// The response trailers.
    trailers: Metadata,
}

impl<'a> InboundReply<'a> {
    /// Construct the reply view for an interceptor callback.
    pub fn new(path: &'a str, headers: Metadata, trailers: Metadata) -> Self {
        Self {
            path,
            headers,
            trailers,
        }
    }

    /// The path of the method, `/package.Service/Method`.
    pub fn path(&self) -> &'a str {
        self.path
    }

    /// The response headers.
    pub fn headers(&self) -> &Metadata {
        &self.headers
    }

    /// The response trailers.
    pub fn trailers(&self) -> &Metadata {
        &self.trailers
    }
}
