//! Integration tests for `AfterCallRequest`.

use std::collections::HashMap;

use edge_transport_grpc_egress_transport::{AfterCallRequest, GrpcResponse};

/// @covers: AfterCallRequest
#[test]
fn test_after_call_request_exposes_mutable_response_happy() {
    let mut response = GrpcResponse {
        body: b"original".to_vec(),
        metadata: HashMap::new(),
    };
    {
        let req = AfterCallRequest {
            response: &mut response,
        };
        req.response.body = b"rewritten".to_vec();
    }
    assert_eq!(response.body, b"rewritten");
}

/// @covers: AfterCallRequest
#[test]
fn test_after_call_request_metadata_mutation_error() {
    let mut response = GrpcResponse {
        body: Vec::new(),
        metadata: HashMap::new(),
    };
    {
        let req = AfterCallRequest {
            response: &mut response,
        };
        req.response
            .metadata
            .insert("x-error".to_string(), "true".to_string());
    }
    assert_eq!(response.metadata.get("x-error"), Some(&"true".to_string()));
}

/// @covers: AfterCallRequest
#[test]
fn test_after_call_request_empty_response_edge() {
    let mut response = GrpcResponse {
        body: Vec::new(),
        metadata: HashMap::new(),
    };
    let req = AfterCallRequest {
        response: &mut response,
    };
    assert!(req.response.body.is_empty());
    assert!(req.response.metadata.is_empty());
}
