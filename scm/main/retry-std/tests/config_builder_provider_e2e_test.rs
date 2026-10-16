#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`ConfigBuilderProvider`] via a test-double
//! implementation.

use grpc_egress_retry::{
    ConfigBuilderProvider, ConfigBuilderRequest, ConfigBuilderResponse, Error,
};
use grpc_egress_retry_std::ApplicationConfigBuilderProvider;
use grpc_egress_retry_std::GrpcRetrySvc;

/// `ApplicationConfigBuilder`'s inner field is crate-private, so this
/// test double delegates construction to the real provider — it still
/// independently verifies the trait contract (error path, default
/// provider) without being a re-test of `GrpcRetrySvc` itself.
struct DelegatingConfigBuilderProvider {
    fail: bool,
}

impl ConfigBuilderProvider for DelegatingConfigBuilderProvider {
    fn create_config_builder(
        &self,
        req: ConfigBuilderRequest,
    ) -> Result<ConfigBuilderResponse, Error> {
        if self.fail {
            return Err(Error::InvalidConfig("mock provider forced failure".into()));
        }
        GrpcRetrySvc.create_config_builder(req)
    }
}

/// @covers: create_config_builder
#[test]
fn test_create_config_builder_delegates_successfully_happy() {
    let provider = DelegatingConfigBuilderProvider { fail: false };
    let resp = provider
        .create_config_builder(ConfigBuilderRequest)
        .expect("happy path");
    resp.builder
        .build_loader()
        .expect("must build a real loader");
}

/// @covers: create_config_builder
#[test]
fn test_create_config_builder_propagates_failure_error() {
    let provider = DelegatingConfigBuilderProvider { fail: true };
    let err = provider
        .create_config_builder(ConfigBuilderRequest)
        .err()
        .expect("forced failure must surface");
    assert!(err.to_string().contains("mock provider forced failure"));
}

/// @covers: create_config_builder
#[test]
fn test_create_config_builder_repeated_calls_are_independent_edge() {
    let provider = DelegatingConfigBuilderProvider { fail: false };
    let first = provider
        .create_config_builder(ConfigBuilderRequest)
        .expect("first call");
    let second = provider
        .create_config_builder(ConfigBuilderRequest)
        .expect("second call");
    first.builder.build_loader().expect("first must build");
    second.builder.build_loader().expect("second must build");
}
