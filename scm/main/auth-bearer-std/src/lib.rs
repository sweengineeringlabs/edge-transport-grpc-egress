//! Standard, dependency-light builders for the bearer-auth domain crate.
//!
//! ```compile_fail
//! use grpc_egress_auth_bearer_std::saf::StdBearerEgressConfigBuilderFactory;
//! ```
//!
#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod saf;
mod std;

pub use crate::saf::{StdBearerEgressConfigBuilderFactory, StdJwtClaimsBuilderFactory};
