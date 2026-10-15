//! Backoff schedule outcome type.
//!
//! The implementation lives in `core/`. This file holds the public type
//! that `core/backoff_scheduler.rs` produces — a [`BackoffSchedule`]
//! descriptor — so the layer-boundary check can find an api/ counterpart
//! for every core/ submodule.

use std::time::Duration;

/// Outcome of one call to the backoff scheduler.
///
/// Construct via the helpers in `core/` — consumers outside the crate
/// don't need to build these directly; the retry loop drives the
/// schedule internally.  The type is public for tests and observability
/// tools that want to inspect the computed delay.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use edge_transport_grpc_egress_retry::BackoffSchedule;
///
/// let schedule = BackoffSchedule::from_duration(Duration::from_millis(200));
/// assert_eq!(schedule.sleep, Duration::from_millis(200));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffSchedule {
    /// How long to sleep before the next retry.
    pub sleep: Duration,
}
