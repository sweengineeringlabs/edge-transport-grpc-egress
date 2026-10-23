//! Immutable values and constants of the breaker domain.

pub mod breaker_egress_log_prefix;
pub mod failure_classifier_log_target;
pub mod grpc_breaker_client_log_target;
pub mod grpc_breaker_config;

pub use breaker_egress_log_prefix::BREAKER_EGRESS_LOG_PREFIX;
pub use failure_classifier_log_target::FAILURE_CLASSIFIER_LOG_TARGET;
pub use grpc_breaker_client_log_target::GRPC_BREAKER_CLIENT_LOG_TARGET;
pub use grpc_breaker_config::GrpcBreakerConfig;
// Moved to edge-transport-breaker-policy (ADR-004/ADR-003): the circuit-state machine's values.
pub use edge_transport_breaker_policy::BreakerState;
// The result of one call as the breaker's state machine counts it.
pub use edge_transport_breaker_policy::Outcome;
