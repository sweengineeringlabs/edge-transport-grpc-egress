//! The `proto-svc` engine behind the `GrpcEgress` port: [`ProtoSvcGrpcEgress`] makes each call through a
//! [`RawTransport`](proto_pattern::RawTransport), and [`EgressCallMapper`] converts between this crate's
//! request, response and error types and the contract's.
pub(crate) mod egress_call_mapper;
pub(crate) mod grpc_egress_builder;
pub(crate) mod grpc_egress_client;
pub(crate) mod payload_source;
pub(crate) mod proto_svc_grpc_egress;
pub(crate) mod tls_file_reader;

pub(crate) use egress_call_mapper::EgressCallMapper;
pub(crate) use grpc_egress_builder::GrpcEgressBuilderImpl;
pub(crate) use proto_svc_grpc_egress::ProtoSvcGrpcEgress;
