//! Config builder for `grpc_egress_transport_std`.

use swe_edge_configbuilder::ConfigBuilderImpl;

/// Wraps the external `swe_edge_configbuilder::ConfigBuilderImpl` so api/
/// never references a foreign crate type directly — conversions and the
/// delegating `build_loader` method live in `std/client/`.
pub struct ApplicationConfigBuilder(pub(crate) ConfigBuilderImpl);
