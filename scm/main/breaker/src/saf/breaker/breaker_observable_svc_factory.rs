//! Composition site for [`BreakerObservable`] — one file per trait keeps wiring focused.

use crate::api::{BreakerObservable, GrpcBreakerClient};

/// Factory that exposes an existing breaker decorator's observability surface.
pub struct BreakerObservableFactory;

impl BreakerObservableFactory {
    /// Upcast an existing [`GrpcBreakerClient`] to its [`BreakerObservable`]
    /// trait object — observability is a property of a live decorator
    /// instance, not something constructed standalone.
    pub fn from_client<T: Send + Sync + 'static>(
        client: GrpcBreakerClient<T>,
    ) -> Box<dyn BreakerObservable> {
        Box::new(client)
    }
}
