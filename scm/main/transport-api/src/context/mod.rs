//! API of the `context` theme: carrying the identity of the caller on a call.

pub mod dto;
pub mod traits;

pub use dto::BuildGrpcCallerIdentityRequest;
pub use traits::{CallerIdentity, GrpcCallerIdentityFactory};
