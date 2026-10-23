//! Minimal example: build a gRPC client for an endpoint, with one option.

use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _client = TonicGrpcEgressBuilderSpiFactory::new("https://localhost:50051")
        .build(BuildGrpcEgressBuilderRequest)
        .with_max_decoding_message_size(8 * 1024 * 1024)
        .build()?;
    println!("client built");
    Ok(())
}
