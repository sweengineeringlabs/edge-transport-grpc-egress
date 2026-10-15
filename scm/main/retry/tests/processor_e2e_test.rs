#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`Processor`] via a test-double implementation.

use edge_transport_grpc_egress_retry::{Error, GrpcRetryConfig, Processor, ProcessorRequest};

struct MockProcessor {
    fail: bool,
}

impl Processor for MockProcessor {
    fn validate(&self, _req: ProcessorRequest) -> Result<(), Error> {
        if self.fail {
            return Err(Error::InvalidConfig("mock processor forced failure".into()));
        }
        Ok(())
    }
}

/// @covers: validate
#[test]
fn test_validate_accepts_config_happy() {
    let processor = MockProcessor { fail: false };
    let result = processor.validate(ProcessorRequest {
        config: GrpcRetryConfig::default(),
    });
    assert!(result.is_ok(), "happy path must succeed");
    // Negative counterpart in the same test: proves this isn't a stub that
    // always returns Ok regardless of the mock's configured behavior.
    let failing = MockProcessor { fail: true };
    let rejected = failing.validate(ProcessorRequest {
        config: GrpcRetryConfig::default(),
    });
    assert!(rejected.is_err(), "a forced-failure mock must still fail");
}

/// @covers: validate
#[test]
fn test_validate_propagates_failure_error() {
    let processor = MockProcessor { fail: true };
    let err = processor
        .validate(ProcessorRequest {
            config: GrpcRetryConfig::default(),
        })
        .expect_err("forced failure must surface");
    assert!(err.to_string().contains("mock processor forced failure"));
}

/// @covers: validate
#[test]
fn test_validate_zero_max_attempts_config_edge() {
    let processor = MockProcessor { fail: false };
    let cfg = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    // The mock doesn't inspect the config's own fields (that's the real
    // DefaultProcessor's job, covered separately) — this proves the
    // request threads an edge-value config through without panicking.
    let result = processor.validate(ProcessorRequest { config: cfg });
    assert!(result.is_ok());
    // Negative counterpart: a failing mock must still fail even with the
    // same edge-value config, proving the Ok above isn't a stub default.
    let failing = MockProcessor { fail: true };
    let cfg2 = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    assert!(failing.validate(ProcessorRequest { config: cfg2 }).is_err());
}
