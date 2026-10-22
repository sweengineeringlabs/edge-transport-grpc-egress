//! `client` theme: the client over the `proto-svc` engine, its builder, and the health-check call.

pub(crate) mod health;
pub(crate) mod protosvc;

pub use health::{HealthCheck, HealthCheckRequest, HealthCheckResponse, HealthStatus};
