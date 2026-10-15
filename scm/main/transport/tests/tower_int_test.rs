#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests that directly exercise the `tower` dependency used by
//! `TonicLbGrpcEgress::call_unary`, which drives a `tonic::transport::Channel`
//! via `tower::ServiceExt::oneshot` (see `spi/loadbalancer/tonic/lb_grpc_egress.rs`).
//!
//! These tests verify the same `Service`/`ServiceExt::oneshot` contract this
//! crate relies on internally, using a minimal `tower::service_fn` stand-in
//! rather than a real tonic Channel.

use tower::{Service as _, ServiceExt as _};

/// @covers: tower::ServiceExt::oneshot — drives a Service to completion for one call
#[tokio::test]
async fn transport_dep_tower_service_ext_oneshot_drives_service_to_completion_int_test() {
    let svc =
        tower::service_fn(|req: u32| async move { Ok::<u32, std::convert::Infallible>(req * 2) });
    let result = svc.oneshot(21).await.expect("service_fn never errors");
    assert_eq!(result, 42, "oneshot must return the service's real output");
}

/// @covers: tower::Service::poll_ready — a ready service accepts a call immediately
#[tokio::test]
async fn transport_dep_tower_service_poll_ready_reports_ready_int_test() {
    let mut svc =
        tower::service_fn(|req: u32| async move { Ok::<u32, std::convert::Infallible>(req) });
    let ready = std::future::poll_fn(|cx| svc.poll_ready(cx)).await;
    assert!(ready.is_ok(), "a stateless service_fn must always be ready");
}
