//! Shared constant for the `BackoffScheduler` implementation — the flat
//! api/ counterpart to the flat `core::retry::backoff::backoff_scheduler`
//! file. The trait itself lives in `api::retry::traits::backoff_scheduler`.

pub mod backoff_scheduler;

pub use backoff_scheduler::BACKOFF_SCHEDULER_LOG_TARGET;
