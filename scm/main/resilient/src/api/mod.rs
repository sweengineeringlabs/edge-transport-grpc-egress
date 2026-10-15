//! API layer — error types, traits, and interface contracts.

mod error;
mod traits;
mod types;

pub use error::ResilientTransportError;
pub use traits::{ConfigBuilderProvider, Processor, Validator};
pub use types::{
    ApplicationConfigBuilder, ConfigBuilderRequest, ConfigBuilderResponse, ConfigValidationRequest,
    DescribeRequest, DescribeResponse, GrpcResilientFacade, GrpcResilientSvcProcessor,
    ResilienceConfig,
};
