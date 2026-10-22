//! Reading a breaker policy; the TOML text is read with `toml`.

pub(crate) mod std_grpc_breaker_config_provider;

pub(crate) use std_grpc_breaker_config_provider::StdGrpcBreakerConfigProvider;
