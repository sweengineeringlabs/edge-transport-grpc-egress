//! Integration tests for `ResilientGrpcClientPort` trait.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;

use futures::future::BoxFuture;

use edge_transport_grpc_egress_transport::{
    CircuitStateRequest, CircuitStateResponse, ConsecutiveFailuresRequest,
    ConsecutiveFailuresResponse, GrpcChannelConfig, GrpcEgress, GrpcEgressError, GrpcRequest,
    GrpcResponse, HealthCheckRequest, KeepAliveConfig, LastErrorRequest, LastErrorResponse,
    MtlsConfig, ResilientGrpcClientPort,
};

/// @covers: ResilientGrpcClientPort is object-safe
#[test]
fn transport_trait_resilient_grpc_client_port_is_object_safe_int_test() {
    fn _assert(_: &dyn ResilientGrpcClientPort) {}
}

/// @covers: GrpcEgress re-export is object-safe
#[test]
fn transport_trait_grpc_egress_is_object_safe_via_resilient_port_int_test() {
    fn _assert(_: &dyn GrpcEgress) {}
}

/// Test-double satisfying the abstract methods, configurable so
/// `circuit_state`/`consecutive_failures`/`last_error` can be exercised for
/// more than one observable state — used both to exercise this trait's
/// default (`Self: Sized`) methods and its 3 observability methods from
/// outside the crate.
// @allow: no_mocks_in_integration — hand-rolled test double, not a mock library.
struct StubResilientClient {
    state: &'static str,
    failure_count: u32,
    last_error: Option<GrpcEgressError>,
}

impl Default for StubResilientClient {
    fn default() -> Self {
        Self {
            state: "Closed",
            failure_count: 0,
            last_error: None,
        }
    }
}

impl GrpcEgress for StubResilientClient {
    fn call_unary(
        &self,
        _request: GrpcRequest,
    ) -> BoxFuture<'_, Result<GrpcResponse, GrpcEgressError>> {
        Box::pin(async {
            Ok(GrpcResponse {
                body: Vec::new(),
                metadata: HashMap::new(),
            })
        })
    }

    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, Result<(), GrpcEgressError>> {
        Box::pin(async { Ok(()) })
    }
}

impl ResilientGrpcClientPort for StubResilientClient {
    fn circuit_state(
        &self,
        _req: CircuitStateRequest,
    ) -> Result<CircuitStateResponse, GrpcEgressError> {
        Ok(CircuitStateResponse { state: self.state })
    }

    fn consecutive_failures(
        &self,
        _req: ConsecutiveFailuresRequest,
    ) -> Result<ConsecutiveFailuresResponse, GrpcEgressError> {
        Ok(ConsecutiveFailuresResponse {
            count: self.failure_count,
        })
    }

    fn last_error(&self, _req: LastErrorRequest) -> Result<LastErrorResponse, GrpcEgressError> {
        Ok(LastErrorResponse {
            error: self.last_error.as_ref().map(|e| match e {
                GrpcEgressError::Unavailable(msg) => GrpcEgressError::Unavailable(msg.clone()),
                other => GrpcEgressError::Internal(other.to_string()),
            }),
        })
    }
}

// ── describe_channel_config ──────────────────────────────────────────────────

/// @covers: describe_channel_config
#[test]
fn test_describe_channel_config_returns_configured_endpoint_happy() {
    let config = GrpcChannelConfig::new("http://127.0.0.1:50051").allow_plaintext();
    assert_eq!(
        <StubResilientClient as ResilientGrpcClientPort>::describe_channel_config(&config),
        "http://127.0.0.1:50051"
    );
}

/// @covers: describe_channel_config
#[test]
fn test_describe_channel_config_empty_endpoint_is_empty_string_error() {
    let config = GrpcChannelConfig::new("").allow_plaintext();
    assert_eq!(
        <StubResilientClient as ResilientGrpcClientPort>::describe_channel_config(&config),
        ""
    );
}

/// @covers: describe_channel_config
#[test]
fn test_describe_channel_config_differs_across_configs_edge() {
    let a = GrpcChannelConfig::new("http://host-a:50051").allow_plaintext();
    let b = GrpcChannelConfig::new("http://host-b:50051").allow_plaintext();
    assert_ne!(
        <StubResilientClient as ResilientGrpcClientPort>::describe_channel_config(&a),
        <StubResilientClient as ResilientGrpcClientPort>::describe_channel_config(&b)
    );
}

// ── default_channel_config_builder ───────────────────────────────────────────

/// @covers: default_channel_config_builder
#[test]
fn test_default_channel_config_builder_missing_endpoint_is_error() {
    let err = <StubResilientClient as ResilientGrpcClientPort>::default_channel_config_builder()
        .build()
        .expect_err("a builder with no endpoint set must fail to build");
    assert!(!err.is_empty(), "error message must be non-empty");
}

/// @covers: default_channel_config_builder
#[test]
fn test_default_channel_config_builder_with_endpoint_builds_valid_config_happy() {
    let config = <StubResilientClient as ResilientGrpcClientPort>::default_channel_config_builder()
        .endpoint("http://127.0.0.1:50051")
        .build()
        .expect("a builder with endpoint set must build");
    assert_eq!(config.endpoint, "http://127.0.0.1:50051");
    // TLS-by-default per `GrpcChannelConfigBuilder::new`'s doc contract.
    assert!(config.tls_required);
}

/// @covers: default_channel_config_builder
#[test]
fn test_default_channel_config_builder_repeated_calls_are_independent_edge() {
    let a = <StubResilientClient as ResilientGrpcClientPort>::default_channel_config_builder();
    let b = <StubResilientClient as ResilientGrpcClientPort>::default_channel_config_builder();
    assert_eq!(a.build().is_err(), b.build().is_err());
}

// ── describe_keep_alive ───────────────────────────────────────────────────────

/// @covers: describe_keep_alive
#[test]
fn test_describe_keep_alive_returns_configured_interval_happy() {
    let cfg = KeepAliveConfig {
        interval: std::time::Duration::from_secs(10),
        timeout: std::time::Duration::from_secs(20),
        permit_without_calls: false,
    };
    assert_eq!(
        <StubResilientClient as ResilientGrpcClientPort>::describe_keep_alive(cfg),
        std::time::Duration::from_secs(10)
    );
}

/// @covers: describe_keep_alive
#[test]
fn test_describe_keep_alive_zero_interval_error() {
    let cfg = KeepAliveConfig {
        interval: std::time::Duration::ZERO,
        timeout: std::time::Duration::from_secs(20),
        permit_without_calls: false,
    };
    assert_eq!(
        <StubResilientClient as ResilientGrpcClientPort>::describe_keep_alive(cfg),
        std::time::Duration::ZERO
    );
}

/// @covers: describe_keep_alive
#[test]
fn test_describe_keep_alive_ignores_timeout_field_edge() {
    let short_interval_long_timeout = KeepAliveConfig {
        interval: std::time::Duration::from_secs(1),
        timeout: std::time::Duration::from_secs(999),
        permit_without_calls: true,
    };
    // Proves the extraction reads `interval`, not `timeout` or any other field.
    assert_eq!(
        <StubResilientClient as ResilientGrpcClientPort>::describe_keep_alive(
            short_interval_long_timeout
        ),
        std::time::Duration::from_secs(1)
    );
}

// ── describe_mtls ─────────────────────────────────────────────────────────────

/// @covers: describe_mtls
#[test]
fn test_describe_mtls_returns_configured_cert_path_happy() {
    let cfg = MtlsConfig {
        cert_pem_path: "/etc/certs/client.pem".to_string(),
        key_pem_path: "/etc/certs/client.key".to_string(),
        ca_pem_path: None,
    };
    assert_eq!(
        <StubResilientClient as ResilientGrpcClientPort>::describe_mtls(&cfg),
        "/etc/certs/client.pem"
    );
}

/// @covers: describe_mtls
#[test]
fn test_describe_mtls_empty_cert_path_error() {
    let cfg = MtlsConfig {
        cert_pem_path: String::new(),
        key_pem_path: "/etc/certs/client.key".to_string(),
        ca_pem_path: None,
    };
    assert_eq!(
        <StubResilientClient as ResilientGrpcClientPort>::describe_mtls(&cfg),
        ""
    );
}

/// @covers: describe_mtls
#[test]
fn test_describe_mtls_ignores_key_and_ca_fields_edge() {
    let cfg = MtlsConfig {
        cert_pem_path: "/etc/certs/client.pem".to_string(),
        key_pem_path: "/should/not/be/returned".to_string(),
        ca_pem_path: Some("/should/not/be/returned/either".to_string()),
    };
    // Proves the extraction reads `cert_pem_path`, not `key_pem_path` or `ca_pem_path`.
    assert_eq!(
        <StubResilientClient as ResilientGrpcClientPort>::describe_mtls(&cfg),
        "/etc/certs/client.pem"
    );
}

// ── circuit_state ─────────────────────────────────────────────────────────────

/// @covers: circuit_state
#[test]
fn test_circuit_state_closed_happy() {
    let client = StubResilientClient::default();
    let resp = client
        .circuit_state(CircuitStateRequest)
        .expect("circuit_state should succeed");
    assert_eq!(resp.state, "Closed");
}

/// @covers: circuit_state
#[test]
fn test_circuit_state_open_error() {
    let client = StubResilientClient {
        state: "Open",
        ..StubResilientClient::default()
    };
    let resp = client
        .circuit_state(CircuitStateRequest)
        .expect("circuit_state should succeed even when the circuit is open");
    assert_eq!(resp.state, "Open");
}

/// @covers: circuit_state
#[test]
fn test_circuit_state_half_open_edge() {
    let client = StubResilientClient {
        state: "HalfOpen",
        ..StubResilientClient::default()
    };
    let resp = client
        .circuit_state(CircuitStateRequest)
        .expect("circuit_state should succeed in the half-open transitional state");
    assert_eq!(resp.state, "HalfOpen");
}

// ── consecutive_failures ──────────────────────────────────────────────────────

/// @covers: consecutive_failures
#[test]
fn test_consecutive_failures_zero_when_healthy_happy() {
    let client = StubResilientClient::default();
    let resp = client
        .consecutive_failures(ConsecutiveFailuresRequest)
        .expect("consecutive_failures should succeed");
    assert_eq!(resp.count, 0);
}

/// @covers: consecutive_failures
#[test]
fn test_consecutive_failures_nonzero_after_failures_error() {
    let client = StubResilientClient {
        failure_count: 3,
        ..StubResilientClient::default()
    };
    let resp = client
        .consecutive_failures(ConsecutiveFailuresRequest)
        .expect("consecutive_failures should succeed");
    assert_eq!(resp.count, 3);
}

/// @covers: consecutive_failures
#[test]
fn test_consecutive_failures_saturating_boundary_edge() {
    let client = StubResilientClient {
        failure_count: u32::MAX,
        ..StubResilientClient::default()
    };
    let resp = client
        .consecutive_failures(ConsecutiveFailuresRequest)
        .expect("consecutive_failures should succeed at the u32 boundary");
    assert_eq!(resp.count, u32::MAX);
}

// ── last_error ────────────────────────────────────────────────────────────────

/// @covers: last_error
#[test]
fn test_last_error_none_when_healthy_happy() {
    let client = StubResilientClient::default();
    let resp = client
        .last_error(LastErrorRequest)
        .expect("last_error should succeed");
    assert!(resp.error.is_none());
}

/// @covers: last_error
#[test]
fn test_last_error_some_after_failure_error() {
    let client = StubResilientClient {
        last_error: Some(GrpcEgressError::Unavailable("backend down".to_string())),
        ..StubResilientClient::default()
    };
    let resp = client
        .last_error(LastErrorRequest)
        .expect("last_error should succeed");
    match resp.error {
        Some(GrpcEgressError::Unavailable(msg)) => assert_eq!(msg, "backend down"),
        other => panic!("expected Some(Unavailable), got {other:?}"),
    }
}

/// @covers: last_error
#[test]
fn test_last_error_repeated_calls_are_independent_edge() {
    let client = StubResilientClient {
        last_error: Some(GrpcEgressError::Unavailable("backend down".to_string())),
        ..StubResilientClient::default()
    };
    let first = client.last_error(LastErrorRequest).expect("first call");
    let second = client.last_error(LastErrorRequest).expect("second call");
    assert_eq!(first.error.is_some(), second.error.is_some());
}
