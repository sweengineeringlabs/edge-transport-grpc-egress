//! Composition site for [`RetryInspector`] — one file per trait keeps wiring focused.

use crate::std::default_retry_inspector::DefaultRetryInspector;
use grpc_egress_retry::RetryInspector;

/// Factory for the default [`RetryInspector`].
pub struct RetryInspectorFactory;

impl RetryInspectorFactory {
    /// Construct the default [`RetryInspector`].
    pub fn create() -> Box<dyn RetryInspector> {
        Box::new(DefaultRetryInspector)
    }
}
