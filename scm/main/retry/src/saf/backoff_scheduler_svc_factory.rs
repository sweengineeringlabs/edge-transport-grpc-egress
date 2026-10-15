//! Composition site for [`BackoffScheduler`] — one file per trait keeps wiring focused.

use crate::api::BackoffScheduler;
use crate::core::retry::backoff::default_backoff_scheduler::DefaultBackoffScheduler;

/// Factory for the default [`BackoffScheduler`].
pub struct BackoffSchedulerFactory;

impl BackoffSchedulerFactory {
    /// Construct the default [`BackoffScheduler`].
    pub fn create() -> Box<dyn BackoffScheduler> {
        Box::new(DefaultBackoffScheduler)
    }
}
