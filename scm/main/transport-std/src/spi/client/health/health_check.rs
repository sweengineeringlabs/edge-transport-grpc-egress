//! `HealthCheck` -- the `grpc.health.v1.Health/Check` method.

use proto_pattern::{Method, UnaryMethod};

use super::health_check_request::HealthCheckRequest;
use super::health_check_response::HealthCheckResponse;

/// The standard health-checking call: `client.call::<HealthCheck>(request)` asks a server whether a service is up. It
/// asks the service; it does not only open a connection.
pub struct HealthCheck;

impl Method for HealthCheck {
    type Request = HealthCheckRequest;
    type Response = HealthCheckResponse;
    const PATH: &'static str = "/grpc.health.v1.Health/Check";
}

impl UnaryMethod for HealthCheck {}
