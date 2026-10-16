//! `impl` blocks for [`GrpcRequest`] and [`GrpcRequestBuilder`]. `GrpcRequest` is declared in
//! `api/client/dto/`, the builder in `grpc_request_builder.rs` beside this file.

use crate::std::GrpcRequestBuilder;
use std::collections::HashMap;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use grpc_egress_transport::GrpcRequest;

/// Constructors and fluent setters of [`GrpcRequest`], which is plain data in the domain crate.
/// Bring the trait into scope to write `GrpcRequest::new(method, body, deadline)`.
pub trait GrpcRequestFactory: Sized {
    /// Create a new request for `method` with a raw protobuf `body` and a mandatory per-call
    /// `deadline`.
    ///
    /// `deadline` is a required positional argument: there is no overload without it and no
    /// default.
    fn new(method: impl Into<String>, body: Vec<u8>, deadline: Duration) -> Self;

    /// Override the entire metadata block.
    fn with_metadata(self, metadata: HashMap<String, String>) -> Self;

    /// Add a single metadata header key/value pair.
    fn with_header(self, name: impl Into<String>, value: impl Into<String>) -> Self;

    /// Attach a caller-supplied cancellation token. Firing the token aborts the in-flight
    /// request from the caller's side.
    fn with_cancellation(self, token: CancellationToken) -> Self;
}

impl GrpcRequestFactory for GrpcRequest {
    fn new(method: impl Into<String>, body: Vec<u8>, deadline: Duration) -> Self {
        Self {
            method: method.into(),
            body,
            metadata: HashMap::new(),
            deadline,
            cancellation: None,
        }
    }

    fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(name.into(), value.into());
        self
    }

    fn with_cancellation(mut self, token: CancellationToken) -> Self {
        self.cancellation = Some(token);
        self
    }
}

impl GrpcRequestBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the gRPC method path (e.g. `/pkg.Service/Method`).
    pub fn method(mut self, v: impl Into<String>) -> Self {
        self.method = Some(v.into());
        self
    }
    /// Set the encoded request payload bytes.
    pub fn body(mut self, v: Vec<u8>) -> Self {
        self.body = v;
        self
    }
    /// Set the per-request deadline.
    pub fn deadline(mut self, v: std::time::Duration) -> Self {
        self.deadline = Some(v);
        self
    }
    /// Attach outbound metadata headers.
    pub fn metadata(mut self, v: HashMap<String, String>) -> Self {
        self.metadata = v;
        self
    }
    /// Attach a cancellation token to abort the in-flight request.
    pub fn cancellation_token(mut self, v: tokio_util::sync::CancellationToken) -> Self {
        self.cancellation_token = Some(v);
        self
    }

    /// Build the [`GrpcRequest`]. Returns `Err` when method or deadline is unset.
    pub fn build(self) -> Result<GrpcRequest, String> {
        let method = self.method.ok_or("method required")?;
        let deadline = self.deadline.ok_or("deadline required")?;
        let mut req = GrpcRequest::new(method, self.body, deadline);
        req.metadata = self.metadata;
        if let Some(token) = self.cancellation_token {
            req = req.with_cancellation(token);
        }
        Ok(req)
    }
}
