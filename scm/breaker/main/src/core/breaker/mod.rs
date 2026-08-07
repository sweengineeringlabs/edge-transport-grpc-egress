//! Circuit-breaker implementation — the core/ counterpart to `api::breaker`.

pub(crate) mod breaker_decorator;
pub(crate) mod breaker_egress;
pub(crate) mod config_builder_provider;
pub(crate) mod default_processor;
pub(crate) mod default_validator;
pub(crate) mod failure_classifier;
pub(crate) mod grpc;
pub(crate) mod grpc_breaker_facade;
