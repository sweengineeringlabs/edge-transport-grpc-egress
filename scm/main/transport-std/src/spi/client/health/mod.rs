//! The standard health-check call (`grpc.health.v1.Health/Check`) and its messages, written with `prost`.

pub(crate) mod health_check;
pub(crate) mod health_check_request;
pub(crate) mod health_check_response;
pub(crate) mod health_status;

pub use health_check::HealthCheck;
pub use health_check_request::HealthCheckRequest;
pub use health_check_response::HealthCheckResponse;
pub use health_status::HealthStatus;
