//! Integration tests for `CallStreamRequest`.

use std::collections::HashMap;

use edge_transport_grpc_egress_transport::{CallStreamRequest, GrpcMessageStreamResponse};
use futures::stream;

/// @covers: CallStreamRequest
#[test]
fn test_call_stream_request_carries_method_and_metadata_happy() {
    let mut metadata = HashMap::new();
    metadata.insert("x-trace-id".to_string(), "abc123".to_string());
    let req = CallStreamRequest {
        method: "pkg.Service/Method".to_string(),
        metadata: metadata.clone(),
        messages: GrpcMessageStreamResponse {
            stream: Box::pin(stream::iter(Vec::<
                edge_transport_grpc_egress_transport::GrpcEgressResult<Vec<u8>>,
            >::new())),
        },
    };
    assert_eq!(req.method, "pkg.Service/Method");
    assert_eq!(req.metadata, metadata);
}

/// @covers: CallStreamRequest
#[test]
fn test_call_stream_request_empty_method_error() {
    let req = CallStreamRequest {
        method: String::new(),
        metadata: HashMap::new(),
        messages: GrpcMessageStreamResponse {
            stream: Box::pin(stream::iter(Vec::<
                edge_transport_grpc_egress_transport::GrpcEgressResult<Vec<u8>>,
            >::new())),
        },
    };
    assert!(
        req.method.is_empty(),
        "an empty method path is representable, callers must validate before dispatch"
    );
}

/// @covers: CallStreamRequest
#[test]
fn test_call_stream_request_empty_metadata_edge() {
    let req = CallStreamRequest {
        method: "pkg.Service/Method".to_string(),
        metadata: HashMap::new(),
        messages: GrpcMessageStreamResponse {
            stream: Box::pin(stream::iter(Vec::<
                edge_transport_grpc_egress_transport::GrpcEgressResult<Vec<u8>>,
            >::new())),
        },
    };
    assert!(req.metadata.is_empty());
}
