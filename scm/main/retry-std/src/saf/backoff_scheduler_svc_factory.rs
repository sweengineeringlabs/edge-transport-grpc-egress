//! Composition site for [`BackoffScheduler`] — one file per trait keeps wiring focused.

use crate::std::default_backoff_scheduler::DefaultBackoffScheduler;
use grpc_egress_retry::BackoffScheduler;

/// Factory for the default [`BackoffScheduler`].
pub struct BackoffSchedulerFactory;

impl BackoffSchedulerFactory {
    /// Construct the default [`BackoffScheduler`].
    pub fn create() -> Box<dyn BackoffScheduler> {
        Box::new(DefaultBackoffScheduler)
    }
}
