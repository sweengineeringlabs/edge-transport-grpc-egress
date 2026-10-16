//! API of the `interceptor` theme: code that runs before and after every call.

pub mod dto;
pub mod traits;
pub mod vo;

pub use dto::AfterCallRequest;
pub use traits::GrpcEgressInterceptor;
pub use vo::TraceContextSource;
