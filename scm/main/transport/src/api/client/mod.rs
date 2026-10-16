//! API layer -- the `client` theme: building a gRPC egress client.
//!
//! The builder trait and its factory trait are here, with the request envelope of the factory.
//! The client itself (`GrpcEgress`) and its request and response types move into this theme with
//! the split of the flat `traits/` and `types/` folders (issue 17).

pub mod dto;
pub mod traits;
pub mod vo;

pub use dto::BuildGrpcEgressBuilderRequest;
pub use traits::{GrpcEgressBuilder, GrpcEgressBuilderFactory};
pub use vo::{BackendTarget, BalancingStrategy, LoadbalancerSettings};
