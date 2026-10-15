//! Circuit-breaker trait contracts.

pub mod breaker_decorator;
pub mod breaker_observable;
pub mod config_builder_provider;
pub mod processor;
pub mod validator;

pub use breaker_decorator::BreakerDecorator;
pub use breaker_observable::BreakerObservable;
pub use config_builder_provider::ConfigBuilderProvider;
pub use processor::Processor;
pub use validator::Validator;

// Moved to edge-transport-breaker-policy (ADR-004/ADR-003): protocol-agnostic contracts.
pub use edge_transport_breaker_policy::{BreakerTransition, FailureClassifier};
