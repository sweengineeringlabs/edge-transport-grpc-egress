//! `dto` of the health-check call: its request and response messages.

pub(crate) mod health_check_request;
pub(crate) mod health_check_response;

pub use health_check_request::HealthCheckRequest;
pub use health_check_response::HealthCheckResponse;
