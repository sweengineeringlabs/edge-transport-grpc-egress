//! The standard gRPC health check as a typed client method.

use proto_pattern::{Method, UnaryMethod};

use super::dto::{HealthCheckRequest, HealthCheckResponse};

/// The `grpc.health.v1.Health/Check` method.
pub struct HealthCheck;

impl Method for HealthCheck {
    type Request = HealthCheckRequest;
    type Response = HealthCheckResponse;
    const PATH: &'static str = "/grpc.health.v1.Health/Check";
}

impl UnaryMethod for HealthCheck {}
