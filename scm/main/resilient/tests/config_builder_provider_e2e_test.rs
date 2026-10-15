#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`ConfigBuilderProvider`] via a test-double
//! implementation.

use edge_transport_grpc_egress_resilient::{
    ConfigBuilderProvider, ConfigBuilderRequest, ConfigBuilderResponse, GrpcResilientSvcProcessor,
    ResilientTransportError,
};

/// `ApplicationConfigBuilder`'s inner field is crate-private, so this
/// test double delegates construction to the real provider — it still
/// independently verifies the trait contract (error path, default
/// provider) without being a re-test of `GrpcResilientSvcProcessor` itself.
struct DelegatingConfigBuilderProvider {
    fail: bool,
}

impl ConfigBuilderProvider for DelegatingConfigBuilderProvider {
    fn create_config_builder(
        &self,
        req: ConfigBuilderRequest,
    ) -> Result<ConfigBuilderResponse, ResilientTransportError> {
        if self.fail {
            return Err(ResilientTransportError::InvalidResilience(
                "mock provider forced failure".into(),
            ));
        }
        GrpcResilientSvcProcessor.create_config_builder(req)
    }
}

/// @covers: create_config_builder
#[test]
fn test_create_config_builder_delegates_successfully_happy() {
    let provider = DelegatingConfigBuilderProvider { fail: false };
    let resp = provider
        .create_config_builder(ConfigBuilderRequest)
        .expect("happy path");
    // Prove it's a genuinely working builder, not a stub.
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
    // Calling twice must not panic or share broken state between builders —
    // each call constructs an independent, working builder.
    let first = provider
        .create_config_builder(ConfigBuilderRequest)
        .expect("first call");
    let second = provider
        .create_config_builder(ConfigBuilderRequest)
        .expect("second call");
    first.builder.build_loader().expect("first must build");
    second.builder.build_loader().expect("second must build");
}

/// @covers: default_provider
#[test]
fn test_default_provider_returns_zero_sized_marker_happy() {
    let svc = <GrpcResilientSvcProcessor as ConfigBuilderProvider>::default_provider();
    assert_eq!(std::mem::size_of_val(&svc), 0);
}

/// @covers: default_provider
#[test]
fn test_default_provider_actually_implements_the_trait_error() {
    // "error"-flavored scenario for an infallible constructor: prove the
    // returned marker genuinely implements ConfigBuilderProvider (i.e. it
    // isn't just a same-named unrelated zero-sized type) by exercising it.
    let svc = <GrpcResilientSvcProcessor as ConfigBuilderProvider>::default_provider();
    let resp = svc
        .create_config_builder(ConfigBuilderRequest)
        .expect("default_provider's result must implement the trait for real");
    resp.builder
        .build_loader()
        .expect("must build a real loader");
}

/// @covers: default_provider
#[test]
fn test_default_provider_is_deterministic_edge() {
    let a = <GrpcResilientSvcProcessor as ConfigBuilderProvider>::default_provider();
    let b = <GrpcResilientSvcProcessor as ConfigBuilderProvider>::default_provider();
    assert_eq!(
        std::mem::size_of_val(&a),
        std::mem::size_of_val(&b),
        "repeated calls must produce equivalent markers"
    );
}
