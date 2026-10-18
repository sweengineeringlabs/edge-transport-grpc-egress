//! The standard health-check call (`grpc.health.v1.Health/Check`) and its messages, written with `prost`.

pub(crate) mod dto;
pub(crate) mod health_check;
pub(crate) mod vo;

pub use dto::{HealthCheckRequest, HealthCheckResponse};
pub use health_check::HealthCheck;
pub use vo::HealthStatus;
