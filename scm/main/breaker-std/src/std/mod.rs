//! Implementations of the breaker contracts and the types that carry behaviour.

pub(crate) mod breaker_egress;
pub(crate) mod failure_classifier;
pub(crate) mod grpc_breaker_client;
pub(crate) mod grpc_breaker_config;

pub use grpc_breaker_config::GrpcBreakerConfigProvider;

pub(crate) use grpc_breaker_client::GrpcBreakerClient;
