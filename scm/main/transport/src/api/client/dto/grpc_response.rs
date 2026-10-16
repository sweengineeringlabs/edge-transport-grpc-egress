//! gRPC response envelope.

use std::collections::HashMap;

/// A gRPC response envelope.
#[derive(Debug, Clone)]
pub struct GrpcResponse {
    /// The raw decoded response payload bytes.
    pub body: Vec<u8>,
    /// Trailing metadata headers returned by the server.
    pub metadata: HashMap<String, String>,
}
