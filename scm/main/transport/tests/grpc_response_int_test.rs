//! Integration tests for `api/value/grpc/grpc_response.rs`.

use std::collections::HashMap;

use edge_transport_grpc_egress_transport::GrpcResponse;

#[test]
fn transport_struct_grpc_response_holds_body_bytes_int_test() {
    let resp = GrpcResponse {
        body: vec![0x08, 0x01],
        metadata: HashMap::new(),
    };
    assert_eq!(resp.body, vec![0x08, 0x01]);
}
