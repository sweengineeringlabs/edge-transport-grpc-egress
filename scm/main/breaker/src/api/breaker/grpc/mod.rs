//! gRPC breaker client/config grouping — the api/ counterpart to
//! `core::breaker::grpc`.

pub mod grpc_breaker_client;

pub use grpc_breaker_client::GRPC_BREAKER_CLIENT_LOG_TARGET;
