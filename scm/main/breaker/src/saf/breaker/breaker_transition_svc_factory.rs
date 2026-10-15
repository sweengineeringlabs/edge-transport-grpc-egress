//! Composition site for [`BreakerTransition`] — one file per trait keeps
//! wiring focused.

use edge_transport_breaker::DefaultBreakerTransition;

use crate::api::BreakerTransition;

/// Factory for the default [`BreakerTransition`].
pub struct BreakerTransitionFactory;

impl BreakerTransitionFactory {
    /// Construct the default [`BreakerTransition`].
    pub fn create() -> Box<dyn BreakerTransition> {
        Box::new(DefaultBreakerTransition)
    }
}
