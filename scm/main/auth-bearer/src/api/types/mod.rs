//! Types — behavioural type declarations whose impl blocks live in
//! `core/` or `spi/`.

pub mod application_config_builder;
pub mod bearer_egress_config;
pub mod bearer_egress_config_builder;
pub mod bearer_egress_interceptor;
pub mod bearer_secret;
pub mod grpc_auth_bearer_svc;
pub mod jwt_claims_builder;
pub mod metadata_keys;

pub use application_config_builder::ApplicationConfigBuilder;
pub use bearer_egress_config::BearerEgressConfig;
pub use bearer_egress_config_builder::BearerEgressConfigBuilder;
pub use bearer_egress_interceptor::BearerEgressInterceptor;
pub use bearer_secret::BearerSecret;
pub use grpc_auth_bearer_svc::GrpcAuthBearerSvc;
pub use jwt_claims_builder::JwtClaimsBuilder;
pub use metadata_keys::{AUTHORIZATION_HEADER, EXTRACTED_BEARER_SUBJECT};

// Re-export JwtClaims from shared security layer (ADR-015 Tier 0)
pub use swe_edge_security::JwtClaims;
