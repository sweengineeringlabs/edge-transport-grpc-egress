//! Trait contracts of the breaker domain.

pub mod breaker_observable;
pub mod failure_classifier;
pub mod grpc_breaker_config_provider;
pub mod grpc_breaker_config_provider_factory;
pub mod grpc_breaker_factory;

pub use breaker_observable::BreakerObservable;
pub use failure_classifier::FailureClassifier;
pub use grpc_breaker_config_provider::GrpcBreakerConfigProvider;
pub use grpc_breaker_config_provider_factory::GrpcBreakerConfigProviderFactory;
pub use grpc_breaker_factory::GrpcBreakerFactory;
