//! `traits` of the `interceptor` theme.

pub mod grpc_egress_interceptor;
pub mod grpc_egress_interceptor_factory;

pub use grpc_egress_interceptor::GrpcEgressInterceptor;
pub use grpc_egress_interceptor_factory::GrpcEgressInterceptorFactory;
