//! SAF layer — gRPC public facade.

mod grpc;
mod processor_svc_factory;
mod resilient_grpc_client_port_svc_factory;
mod transport_construction_svc_factory;

pub use grpc::GrpcEgressFactory;
pub use grpc::GrpcEgressInterceptorFactory;
pub use processor_svc_factory::ProcessorFactory;
pub use resilient_grpc_client_port_svc_factory::ResilientGrpcClientPortFactory;
pub use transport_construction_svc_factory::TransportConstruction;

#[cfg(feature = "prost")]
pub use grpc::GrpcEgressProstCodecFactory;
