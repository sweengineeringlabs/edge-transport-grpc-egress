//! API layer — config, error, and contracts for the outbound bearer interceptor.

mod error;
mod traits;
mod types;

pub use error::BearerAuthError;
pub use traits::{Config, Processor, Validator};
pub use types::{
    BearerEgressConfig, BearerEgressConfigBuilder, BearerEgressInterceptor, BearerSecret,
    JwtClaims, JwtClaimsBuilder, AUTHORIZATION_HEADER, EXTRACTED_BEARER_SUBJECT,
};

pub use types::GrpcAuthBearerSvc;
