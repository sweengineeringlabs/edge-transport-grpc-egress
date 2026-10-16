//! Immutable values and constants of the bearer-auth domain.

pub mod bearer_egress_config;
pub mod bearer_secret;
pub mod metadata_keys;

pub use bearer_egress_config::BearerEgressConfig;
pub use bearer_secret::BearerSecret;
pub use metadata_keys::{AUTHORIZATION_HEADER, EXTRACTED_BEARER_SUBJECT};
// The claims type of the shared security layer (ADR-015 Tier 0).
pub use swe_edge_security::JwtClaims;
