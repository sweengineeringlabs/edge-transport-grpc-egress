//! `channel` theme: the standard builder of the channel configuration, and the reading of the TLS files it names.

pub(crate) mod std_grpc_channel_config_builder;
pub(crate) mod std_tls_settings_resolver;

pub(crate) use std_grpc_channel_config_builder::StdGrpcChannelConfigBuilder;
pub(crate) use std_tls_settings_resolver::StdTlsSettingsResolver;
