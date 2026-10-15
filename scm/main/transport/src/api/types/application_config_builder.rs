//! Config builder for `edge_transport_grpc_egress_transport`.

use swe_edge_configbuilder::ConfigBuilderImpl;

/// Wraps the external `swe_edge_configbuilder::ConfigBuilderImpl` so api/
/// never references a foreign crate type directly — conversions and the
/// delegating `build_loader` method live in `core/`.
pub struct ApplicationConfigBuilder(pub(crate) ConfigBuilderImpl);
