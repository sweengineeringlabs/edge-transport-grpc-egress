//! API of the `interceptor` theme: code that runs before and after every call.

pub mod dto;
pub mod traits;
pub mod vo;

pub use dto::{AfterCallRequest, BuildGrpcEgressInterceptorRequest};
pub use traits::{GrpcEgressInterceptor, GrpcEgressInterceptorFactory};
pub use vo::TraceContextSource;
