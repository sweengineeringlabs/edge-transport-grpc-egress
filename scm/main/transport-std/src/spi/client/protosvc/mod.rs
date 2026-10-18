//! The `proto-svc` engine behind the builder: [`GrpcEgressBuilderImpl`] builds one client per endpoint from
//! `proto-svc`'s `DualTransportFactory` and [`EndpointTransport`] spreads calls over them.
pub(crate) mod endpoint_transport;
pub(crate) mod grpc_egress_builder;

pub(crate) use endpoint_transport::EndpointTransport;
pub(crate) use grpc_egress_builder::GrpcEgressBuilderImpl;
