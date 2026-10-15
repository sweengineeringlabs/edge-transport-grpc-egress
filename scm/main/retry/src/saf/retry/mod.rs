//! SAF layer — composition sites for the retry-decoration and
//! retry-decision-inspection traits.

mod retry_decorator_svc_factory;
mod retry_inspector_svc_factory;

pub use retry_decorator_svc_factory::RetryDecoratorFactory;
pub use retry_inspector_svc_factory::RetryInspectorFactory;
