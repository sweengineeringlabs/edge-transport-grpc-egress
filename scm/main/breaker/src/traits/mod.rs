//! Trait contracts of the breaker domain.

pub mod breaker_observable;
pub mod grpc_breaker_factory;

pub use breaker_observable::BreakerObservable;
pub use grpc_breaker_factory::GrpcBreakerFactory;
