//! `StdBearerEgressConfigBuilderFactory` -- creates the builder of the outbound bearer config.

use grpc_egress_auth_bearer_api::{
    BearerEgressConfigBuilder, BearerEgressConfigBuilderFactory,
    BuildBearerEgressConfigBuilderRequest,
};

use crate::std::StdBearerEgressConfigBuilder;

/// Creates builders of the outbound bearer config.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdBearerEgressConfigBuilderFactory;

impl BearerEgressConfigBuilderFactory for StdBearerEgressConfigBuilderFactory {
    fn build(
        &self,
        _req: BuildBearerEgressConfigBuilderRequest,
    ) -> impl BearerEgressConfigBuilder + use<> {
        StdBearerEgressConfigBuilder::default()
    }
}
