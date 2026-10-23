//! The breaker decorator, over `tokio`, `futures`, `tracing` and the breaker policy crates.

pub(crate) mod breaker_egress;
pub(crate) mod grpc_breaker_client;
pub(crate) mod std_failure_classifier;

pub(crate) use grpc_breaker_client::GrpcBreakerClient;
