//! `GrpcRequestBuilder` — builder for [`grpc_egress_transport::GrpcRequest`].

use std::collections::HashMap;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

/// Builder for [`grpc_egress_transport::GrpcRequest`].
#[derive(Debug, Default)]
pub struct GrpcRequestBuilder {
    pub(crate) method: Option<String>,
    pub(crate) body: Vec<u8>,
    pub(crate) deadline: Option<Duration>,
    pub(crate) metadata: HashMap<String, String>,
    pub(crate) cancellation_token: Option<CancellationToken>,
}
