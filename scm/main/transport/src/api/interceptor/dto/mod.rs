//! `dto` of the `interceptor` theme.

pub mod build_grpc_egress_interceptor_request;
pub mod inbound_reply;

pub use build_grpc_egress_interceptor_request::BuildGrpcEgressInterceptorRequest;
pub use inbound_reply::InboundReply;
