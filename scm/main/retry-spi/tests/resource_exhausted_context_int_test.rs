//! Integration tests for `ResourceExhaustedContext`.
//!
//! Exercises the `classify` method that discriminates `RESOURCE_EXHAUSTED`
//! gRPC status messages into Capacity / RateLimit / HardQuota contexts.

/// @covers: ResourceExhaustedContext::classify — quota message classifies as HardQuota
use grpc_egress_retry_api::ResourceExhaustedContext;
use grpc_egress_retry_api::RetryClassifier as _;
#[test]
fn retry_struct_resource_exhausted_context_quota_message_classifies_as_hard_quota_int_test() {
    assert_eq!(
        classifier().classify_resource_exhausted("billing quota exceeded"),
        ResourceExhaustedContext::HardQuota,
        "billing/quota keyword must classify as HardQuota"
    );
}

/// @covers: ResourceExhaustedContext::classify — rate-limit message classifies as RateLimit
#[test]
fn retry_struct_resource_exhausted_context_rate_message_classifies_as_rate_limit_int_test() {
    assert_eq!(
        classifier().classify_resource_exhausted("too many requests"),
        ResourceExhaustedContext::RateLimit,
        "rate/throttle keyword must classify as RateLimit"
    );
}

/// @covers: ResourceExhaustedContext::classify — unknown message classifies as Capacity
#[test]
fn retry_struct_resource_exhausted_context_unknown_message_classifies_as_capacity_int_test() {
    assert_eq!(
        classifier().classify_resource_exhausted("server overloaded"),
        ResourceExhaustedContext::Capacity,
        "unknown message must default to Capacity"
    );
}

/// @covers: ResourceExhaustedContext is accessible — all three variants are pairwise distinct
#[test]
fn retry_struct_resource_exhausted_context_is_accessible_int_test() {
    assert_ne!(
        ResourceExhaustedContext::Capacity,
        ResourceExhaustedContext::RateLimit
    );
    assert_ne!(
        ResourceExhaustedContext::RateLimit,
        ResourceExhaustedContext::HardQuota
    );
    assert_ne!(
        ResourceExhaustedContext::Capacity,
        ResourceExhaustedContext::HardQuota
    );
}

fn classifier() -> impl grpc_egress_retry_api::RetryClassifier {
    use grpc_egress_retry_api::RetryClassifierFactory;
    grpc_egress_retry_std::StdRetryClassifierFactory
        .build(grpc_egress_retry_api::BuildRetryClassifierRequest)
}
