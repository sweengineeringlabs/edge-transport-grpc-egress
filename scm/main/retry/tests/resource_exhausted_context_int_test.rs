//! Integration tests for `ResourceExhaustedContext`.
//!
//! Exercises the `classify` method that discriminates `RESOURCE_EXHAUSTED`
//! gRPC status messages into Capacity / RateLimit / HardQuota contexts.

use edge_transport_grpc_egress_retry::ResourceExhaustedContext;

/// @covers: ResourceExhaustedContext::classify — quota message classifies as HardQuota
#[test]
fn retry_struct_resource_exhausted_context_quota_message_classifies_as_hard_quota_int_test() {
    assert_eq!(
        ResourceExhaustedContext::classify("billing quota exceeded"),
        ResourceExhaustedContext::HardQuota,
        "billing/quota keyword must classify as HardQuota"
    );
}

/// @covers: ResourceExhaustedContext::classify — rate-limit message classifies as RateLimit
#[test]
fn retry_struct_resource_exhausted_context_rate_message_classifies_as_rate_limit_int_test() {
    assert_eq!(
        ResourceExhaustedContext::classify("too many requests"),
        ResourceExhaustedContext::RateLimit,
        "rate/throttle keyword must classify as RateLimit"
    );
}

/// @covers: ResourceExhaustedContext::classify — unknown message classifies as Capacity
#[test]
fn retry_struct_resource_exhausted_context_unknown_message_classifies_as_capacity_int_test() {
    assert_eq!(
        ResourceExhaustedContext::classify("server overloaded"),
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
