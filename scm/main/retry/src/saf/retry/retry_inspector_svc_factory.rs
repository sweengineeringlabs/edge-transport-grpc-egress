//! Composition site for [`RetryInspector`] — one file per trait keeps wiring focused.

use crate::api::RetryInspector;
use crate::core::retry::default_retry_inspector::DefaultRetryInspector;

/// Factory for the default [`RetryInspector`].
pub struct RetryInspectorFactory;

impl RetryInspectorFactory {
    /// Construct the default [`RetryInspector`].
    pub fn create() -> Box<dyn RetryInspector> {
        Box::new(DefaultRetryInspector)
    }
}
