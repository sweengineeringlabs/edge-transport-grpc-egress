//! `ApplicationConfigBuilder` — maps to `config/application.toml`.

use swe_edge_configbuilder::ConfigBuilderImpl;

/// Config builder corresponding to `config/application.toml`.
///
/// Wraps the external `swe_edge_configbuilder::ConfigBuilderImpl` so api/
/// never references a foreign crate type directly. The delegating `build_loader` is in
/// `grpc-egress-retry-std`.
pub struct ApplicationConfigBuilder(pub ConfigBuilderImpl);
