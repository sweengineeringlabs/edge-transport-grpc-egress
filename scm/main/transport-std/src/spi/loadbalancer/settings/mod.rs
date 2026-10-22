//! Reading the load balancer settings from a configuration file, with `toml`.

pub(crate) mod loadbalancer_settings_provider;

pub(crate) use loadbalancer_settings_provider::StdLoadbalancerSettingsProvider;
