//! The retry decorator, over `tokio` and `edge-transport-retry`.

pub(crate) mod grpc_retry_client;
pub(crate) mod retry_transport;

pub(crate) use grpc_retry_client::GrpcRetryClient;
