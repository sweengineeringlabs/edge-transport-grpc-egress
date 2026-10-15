//! `GrpcClientBuilder` — interface counterpart for `core/client/grpc_client_builder.rs`.

/// Builder marker type for gRPC clients.
///
/// The concrete builder lives in `core/`; callers use
/// [`crate::TransportConstruction::create_transport_from_config`] instead of direct construction.
pub struct GrpcClientBuilder;
