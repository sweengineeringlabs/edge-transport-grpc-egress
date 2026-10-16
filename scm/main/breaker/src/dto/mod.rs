//! Request and response envelopes of the breaker domain.

pub mod config_builder_request;
pub mod config_builder_response;
pub mod config_validation_request;
pub mod describe_request;
pub mod describe_response;
pub mod observe_state_request;
pub mod observe_state_response;
pub mod wrap_breaker_request;
pub mod wrap_breaker_response;

pub use config_builder_request::ConfigBuilderRequest;
pub use config_builder_response::ConfigBuilderResponse;
pub use config_validation_request::ConfigValidationRequest;
pub use describe_request::DescribeRequest;
pub use describe_response::DescribeResponse;
pub use observe_state_request::ObserveStateRequest;
pub use observe_state_response::ObserveStateResponse;
pub use wrap_breaker_request::WrapBreakerRequest;
pub use wrap_breaker_response::WrapBreakerResponse;
// Moved to edge-transport-breaker-policy (ADR-004/ADR-003): the circuit-state machine's requests and
// responses are protocol-agnostic.
pub use edge_transport_breaker_policy::{
    AdmitRequest, AdmitResponse, ClassifyRequest, ClassifyResponse, RecordOutcomeRequest,
    RecordOutcomeResponse,
};
