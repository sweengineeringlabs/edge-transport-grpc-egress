//! `BearerEgressConfigBuilderFactory` -- creates the builder of the outbound bearer config.

use crate::{BearerEgressConfigBuilder, BuildBearerEgressConfigBuilderRequest};

/// Creates a [`BearerEgressConfigBuilder`].
pub trait BearerEgressConfigBuilderFactory {
    /// A new, empty builder. Each call returns an independent builder.
    fn build(
        &self,
        req: BuildBearerEgressConfigBuilderRequest,
    ) -> impl BearerEgressConfigBuilder + use<Self>;
}
