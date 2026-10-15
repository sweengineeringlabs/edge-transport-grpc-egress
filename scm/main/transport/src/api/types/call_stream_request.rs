//! Request for [`crate::api::GrpcEgress::call_stream`],
//! [`crate::api::GrpcEgress::call_client_stream`], and
//! [`crate::api::GrpcEgress::call_bidi_stream`] — all three take the
//! same shape (a method path, metadata, and an inbound message stream).

use std::collections::HashMap;

use crate::api::GrpcMessageStreamResponse;

/// Input to a streaming [`crate::api::GrpcEgress`] call.
pub struct CallStreamRequest {
    /// Fully-qualified gRPC method path (e.g. `"pkg.Service/Method"`).
    pub method: String,
    /// Request metadata (headers).
    pub metadata: HashMap<String, String>,
    /// Inbound stream of request messages.
    pub messages: GrpcMessageStreamResponse,
}
