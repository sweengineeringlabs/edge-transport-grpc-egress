//! Primary trait declarations for `edge-transport-grpc-egress-resilient`.
//!
//! | Trait | Contract |
//! |---|---|
//! | [`Processor`] | Primary processing trait for this service_type = "processor" crate |
//! | [`Validator`] | Configuration validation contract |
//! | [`ConfigBuilderProvider`] | Pre-seeded application config builder contract |

pub mod config_builder_provider;
pub mod processor;
pub mod validator;

pub use config_builder_provider::ConfigBuilderProvider;
pub use processor::Processor;
pub use validator::Validator;
