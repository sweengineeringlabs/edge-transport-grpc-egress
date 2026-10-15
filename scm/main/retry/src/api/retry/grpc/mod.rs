//! Shared constant for the `GrpcRetryClient` inherent impl — the flat
//! api/ counterpart to the flat `core::retry::grpc::grpc_retry_client`
//! file. The trait that gives it signature presence lives in
//! `api::retry::traits::retry_decorator`.

pub mod grpc_retry_client;

pub use grpc_retry_client::GRPC_RETRY_CLIENT_LOG_TARGET;
