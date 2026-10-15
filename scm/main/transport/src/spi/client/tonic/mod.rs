//! Hyper/hyper-rustls-backed gRPC client (`TonicGrpcEgress`) and its builders.
//!
//! These are external-library wrapper implementations of the `api/` `GrpcEgress`
//! and `Processor` contracts; they are exported solely through `saf/`.
pub(crate) mod tonic_grpc_egress;
pub(crate) mod tonic_grpc_egress_builder;
pub(crate) mod tonic_grpc_egress_protocol;

pub(crate) use tonic_grpc_egress::TonicGrpcEgress;
pub(crate) use tonic_grpc_egress_protocol::TonicGrpcEgressProtocol;
