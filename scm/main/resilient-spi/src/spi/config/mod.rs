//! Reading a resilience policy; the TOML text is read with `toml`.

pub(crate) mod std_resilience_config_provider;

pub(crate) use std_resilience_config_provider::StdResilienceConfigProvider;
