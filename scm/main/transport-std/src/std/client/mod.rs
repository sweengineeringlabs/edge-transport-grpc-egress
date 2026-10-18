//! `client` theme: the health-check call, and the building of call metadata.

pub(crate) mod dto;
pub(crate) mod health_check;
pub(crate) mod metadata_extender;
pub(crate) mod vo;

pub use dto::{HealthCheckRequest, HealthCheckResponse};
pub use health_check::HealthCheck;
pub(crate) use metadata_extender::MetadataExtender;
pub use vo::HealthStatus;
