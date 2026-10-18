//! `std` layer: the standard implementation, defaults only. It names the domain crate, `core`, `std` and `alloc`, and no
//! third-party crate; code that needs one is in `spi/`.

pub(crate) mod std_bearer_egress_config_builder;
pub(crate) mod std_jwt_claims_builder;

pub(crate) use std_bearer_egress_config_builder::StdBearerEgressConfigBuilder;
pub(crate) use std_jwt_claims_builder::StdJwtClaimsBuilder;
