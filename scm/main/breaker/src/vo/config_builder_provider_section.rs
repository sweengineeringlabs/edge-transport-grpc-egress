//! Shared constant for the config builder provider implementation — the
//! flat api/ counterpart to the flat `core::breaker::config_builder_provider`
//! file.

/// The config section name this crate's builder pre-seeds.
pub const CONFIG_BUILDER_PROVIDER_SECTION: &str = "grpc_breaker";
