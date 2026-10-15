//! Theme-grouped composition sites for the `GrpcEgress` trait family —
//! `grpc_egress_svc_factory.rs`, `grpc_egress_interceptor_svc_factory.rs`,
//! and `grpc_egress_prost_codec_svc_factory.rs` share the `grpc` prefix as
//! flat siblings of `saf/`, so `shared_prefix_grouping` requires grouping
//! them into a subdirectory. Per `saf_trait_svc_correspondence`'s own
//! documented "theme-grouped layout" allowance, each file keeps its full
//! `<trait_snake>_svc_factory.rs` name even when nested.
mod grpc_egress_interceptor_svc_factory;
mod grpc_egress_svc_factory;

pub use grpc_egress_interceptor_svc_factory::GrpcEgressInterceptorFactory;
pub use grpc_egress_svc_factory::GrpcEgressFactory;

#[cfg(feature = "prost")]
mod grpc_egress_prost_codec_svc_factory;
#[cfg(feature = "prost")]
pub use grpc_egress_prost_codec_svc_factory::GrpcEgressProstCodecFactory;
