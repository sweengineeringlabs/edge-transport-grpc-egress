//! Composition site for [`BreakerDecorator`] — one file per trait keeps wiring focused.

use edge_transport_grpc_egress::GrpcEgress;

use crate::api::BreakerDecorator;
use crate::core::breaker::breaker_decorator::DefaultBreakerDecorator;

/// Factory for the default [`BreakerDecorator`].
pub struct BreakerDecoratorFactory;

impl BreakerDecoratorFactory {
    /// Construct the default [`BreakerDecorator`] for inner clients of type `T`.
    pub fn create<T: GrpcEgress + Send + Sync + 'static>() -> Box<dyn BreakerDecorator<T>> {
        Box::new(DefaultBreakerDecorator)
    }
}
