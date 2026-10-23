//! The domain API's typed health payloads preserve the standard gRPC health wire format.

use grpc_egress_transport_api::{
    HealthCheck, HealthCheckRequest, HealthCheckResponse, HealthStatus,
};
use proto_pattern::{Method, Payload};

/// @covers: HealthCheck
#[test]
fn test_health_check_uses_the_standard_unary_method_path() {
    assert_eq!(
        <HealthCheck as Method>::PATH,
        "/grpc.health.v1.Health/Check"
    );
}

/// @covers: HealthCheckRequest
#[test]
fn test_health_check_request_uses_the_grpc_health_protobuf_wire_format() {
    let request = HealthCheckRequest {
        service: "pkg.Service".to_string(),
    };
    let mut bytes = Vec::new();

    assert!(request.encode_into(&mut bytes).is_ok());

    assert_eq!(
        bytes,
        [0x0a, 0x0b, b'p', b'k', b'g', b'.', b'S', b'e', b'r', b'v', b'i', b'c', b'e']
    );
    assert_eq!(HealthCheckRequest::decode(&bytes).ok(), Some(request));
}

/// @covers: HealthCheckRequest
#[test]
fn test_health_check_request_omits_default_and_skips_unknown_fields() {
    let mut bytes = Vec::new();
    assert!(HealthCheckRequest::default()
        .encode_into(&mut bytes)
        .is_ok());
    assert!(bytes.is_empty());

    let decoded = HealthCheckRequest::decode(&[0x18, 0x01, 0x0a, 0x03, b'a', b'b', b'c'])
        .map(|request| request.service)
        .ok();
    assert_eq!(decoded.as_deref(), Some("abc"));
}

/// @covers: HealthCheckResponse, HealthStatus
#[test]
fn test_health_check_response_preserves_unknown_status_values() {
    let response = HealthCheckResponse { status: 42 };
    let mut bytes = Vec::new();
    assert!(response.encode_into(&mut bytes).is_ok());

    assert_eq!(bytes, [0x08, 0x2a]);
    assert_eq!(HealthCheckResponse::decode(&bytes).ok(), Some(response));
    assert_eq!(response.status(), HealthStatus::Unknown);
}

/// @covers: HealthCheckRequest, HealthCheckResponse
#[test]
fn test_health_check_payloads_reject_truncated_fields() {
    assert!(HealthCheckRequest::decode(&[0x0a, 0x02, b'x']).is_err());
    assert!(HealthCheckResponse::decode(&[0x08, 0x80]).is_err());
}
