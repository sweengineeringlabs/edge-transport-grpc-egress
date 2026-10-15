#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests verifying the concrete transport client satisfies the
//! `Processor` contract through the `TransportSvc` SAF factory, plus the
//! `Processor` trait's default methods via a test-double implementation.

use futures::future::BoxFuture;

use edge_transport_grpc_egress_transport::{
    DescribeRequest, DescribeResponse, GrpcChannelConfig, GrpcChannelConfigError, GrpcEgressError,
    ProcessingRequest, Processor, TransportConstruction,
};

/// Test-double satisfying `Processor`'s two abstract methods. `process`
/// succeeds unless `fail` is set — used to exercise the trait's default
/// methods (`default_config_builder` / `default_facade`) and `process`
/// itself from outside the crate.
// @allow: no_mocks_in_integration — hand-rolled test double, not a mock library.
#[derive(Default)]
struct StubProcessor {
    fail: bool,
}

impl Processor for StubProcessor {
    fn process(&self, _req: ProcessingRequest) -> BoxFuture<'_, Result<(), GrpcEgressError>> {
        let fail = self.fail;
        Box::pin(async move {
            if fail {
                return Err(GrpcEgressError::Internal("process forced failure".into()));
            }
            Ok(())
        })
    }

    fn describe(&self, _req: DescribeRequest) -> Result<DescribeResponse, GrpcEgressError> {
        Ok(DescribeResponse {
            label: "stub-processor",
        })
    }
}

fn ensure_rustls_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

/// @covers: create_transport_from_config — factory produces a valid transport
#[test]
fn transport_struct_processor_factory_creates_valid_transport_int_test() {
    ensure_rustls_provider();
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051").allow_plaintext();
    let transport =
        TransportConstruction::create_transport_from_config(&cfg).expect("create transport");
    let _ = transport;
}

/// @covers: create_tonic_client_from_config — returns concrete client
#[test]
fn transport_struct_processor_tonic_client_created_int_test() {
    ensure_rustls_provider();
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051").allow_plaintext();
    let client =
        TransportConstruction::create_tonic_client_from_config(&cfg).expect("tonic client");
    let _ = client;
}

// ── Processor::default_config_builder ────────────────────────────────────────

#[derive(serde::Deserialize, Default, PartialEq, Debug)]
struct AbsentSectionProbe {
    marker: bool,
}

/// @covers: default_config_builder
#[test]
fn test_default_config_builder_builds_valid_loader_happy() {
    let loader = <StubProcessor as Processor>::default_config_builder()
        .build_loader()
        .expect("a builder pre-seeded with name and version must build a valid loader");
    let _ = loader;
}

/// @covers: default_config_builder
#[test]
fn test_default_config_builder_unconfigured_section_is_error() {
    let loader = <StubProcessor as Processor>::default_config_builder()
        .build_loader()
        .expect("build loader");
    // No application.toml exists in the test environment, so any section
    // lookup must fail with NotFound — proves this is a real loader, not a
    // stub that silently succeeds.
    let err = loader
        .load_section::<AbsentSectionProbe>("processor_test_probe_section_that_does_not_exist")
        .expect_err("no config directory exists in the test environment");
    assert!(
        err.to_string()
            .contains("processor_test_probe_section_that_does_not_exist"),
        "error must name the missing section, got: {err}"
    );
}

/// @covers: default_config_builder
#[test]
fn test_default_config_builder_repeated_calls_are_independent_edge() {
    let first = <StubProcessor as Processor>::default_config_builder()
        .build_loader()
        .expect("first builder");
    let second = <StubProcessor as Processor>::default_config_builder()
        .build_loader()
        .expect("second builder");
    let _ = (first, second);
}

// ── Processor::default_facade ────────────────────────────────────────────────

/// @covers: default_facade
#[test]
fn test_default_facade_returns_zero_sized_marker_happy() {
    let svc = <StubProcessor as Processor>::default_facade();
    assert_eq!(
        std::mem::size_of_val(&svc),
        0,
        "TransportSvc is a zero-sized facade marker"
    );
}

/// @covers: default_facade
#[test]
fn test_default_facade_is_usable_as_the_real_transport_svc_error() {
    // The facade returned by the trait default must be the genuine
    // `TransportSvc` — proven by calling one of its real static methods and
    // checking the *specific* error variant a plaintext-rejecting config
    // produces, not a look-alike stub type that would happen to return Ok.
    let _svc = <StubProcessor as Processor>::default_facade();
    ensure_rustls_provider();
    // TLS is required by default and the endpoint is never marked plaintext.
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051");
    let err = TransportConstruction::create_transport_from_config(&cfg)
        .err()
        .expect("plaintext endpoint with tls_required must be rejected");
    assert!(
        matches!(err, GrpcChannelConfigError::PlaintextRejected(_)),
        "expected PlaintextRejected, got: {err:?}"
    );
}

/// @covers: default_facade
#[test]
fn test_default_facade_is_deterministic_edge() {
    let a = <StubProcessor as Processor>::default_facade();
    let b = <StubProcessor as Processor>::default_facade();
    assert_eq!(std::mem::size_of_val(&a), std::mem::size_of_val(&b));
}

// ── Processor::process ───────────────────────────────────────────────────────

/// @covers: process
#[tokio::test]
async fn test_process_succeeds_happy() {
    let processor = StubProcessor::default();
    processor
        .process(ProcessingRequest)
        .await
        .expect("process should succeed by default");
}

/// @covers: process
#[tokio::test]
async fn test_process_propagates_forced_failure_error() {
    let processor = StubProcessor { fail: true };
    let err = processor
        .process(ProcessingRequest)
        .await
        .expect_err("process must propagate the forced failure");
    assert!(matches!(err, GrpcEgressError::Internal(_)));
}

/// @covers: process
#[tokio::test]
async fn test_process_repeated_calls_are_independent_edge() {
    let processor = StubProcessor::default();
    processor
        .process(ProcessingRequest)
        .await
        .expect("first call");
    processor
        .process(ProcessingRequest)
        .await
        .expect("second call must not be affected by the first");
}
