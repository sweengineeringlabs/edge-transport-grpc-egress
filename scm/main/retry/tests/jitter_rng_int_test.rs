//! Coverage stub for `src/api/backoff/jitter_rng.rs`.
//!
//! `JitterRng` trait is `pub(crate)` — not part of the public API.
//! The jitter is applied internally during backoff computation.
//! This stub verifies the public types that depend on it compile.

use edge_transport_grpc_egress_retry::GrpcRetryConfig;

/// @covers: JitterRng (internal) — GrpcRetryConfig.jitter_factor is the public knob
#[test]
fn retry_trait_jitter_rng_config_jitter_factor_is_accessible_int_test() {
    let cfg = GrpcRetryConfig::default();
    // jitter_factor is the public knob that governs how JitterRng is used.
    assert!((0.0..=1.0).contains(&cfg.jitter_factor));
}
