//! API of the resilient crate: `dto/`, `vo/`, `error/` and `traits/`.

mod dto;
mod error;
mod traits;
mod vo;

// Flat re-export surface: the other layers and lib.rs reach api/ items only as `crate::api::TypeName`.
pub use dto::{
    ConfigBuilderRequest, ConfigBuilderResponse, ConfigValidationRequest, DescribeRequest,
    DescribeResponse,
};
pub use error::ResilientTransportError;
pub use traits::{ConfigBuilderProvider, Processor, Validator};
pub use vo::ResilienceConfig;
