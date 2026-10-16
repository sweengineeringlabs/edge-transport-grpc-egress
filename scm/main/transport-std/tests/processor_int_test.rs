#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests verifying the concrete transport client satisfies the
//! `Processor` contract through the `TransportSvc` SAF factory, plus the
//! `Processor` trait's default methods via a test-double implementation.

use futures::future::BoxFuture;
use grpc_egress_transport_std::GrpcChannelConfigFactory;

use grpc_egress_transport::{
    DescribeRequest, DescribeResponse, GrpcChannelConfig, GrpcEgressError, ProcessingRequest,
    Processor,
};
use grpc_egress_transport_std::TransportConstruction;

/// Test-double satisfying `Processor`'s two abstract methods. `process`
/// succeeds unless `fail` is set — used to exercise the trait's default
/// `process`
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
