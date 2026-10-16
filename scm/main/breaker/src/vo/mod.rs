//! Immutable values and constants of the breaker domain.

pub mod application_config_builder;
pub mod breaker_decorator_label;
pub mod breaker_egress_log_prefix;
pub mod config_builder_provider_section;
pub mod failure_classifier_log_target;
pub mod grpc_breaker_client_log_target;
pub mod grpc_breaker_config;

pub use application_config_builder::ApplicationConfigBuilder;
pub use breaker_decorator_label::BREAKER_DECORATOR_LABEL;
pub use breaker_egress_log_prefix::BREAKER_EGRESS_LOG_PREFIX;
pub use config_builder_provider_section::CONFIG_BUILDER_PROVIDER_SECTION;
pub use failure_classifier_log_target::FAILURE_CLASSIFIER_LOG_TARGET;
pub use grpc_breaker_client_log_target::GRPC_BREAKER_CLIENT_LOG_TARGET;
pub use grpc_breaker_config::GrpcBreakerConfig;
// Moved to edge-transport-breaker-policy (ADR-004/ADR-003): the circuit-state machine's values.
pub use edge_transport_breaker_policy::{Admission, BreakerState, Outcome};
