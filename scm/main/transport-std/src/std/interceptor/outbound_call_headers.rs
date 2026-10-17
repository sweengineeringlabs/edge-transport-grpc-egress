//! [`OutboundCallHeaders`] -- reading and adding the request headers of an [`OutboundCall`].

use grpc_egress_transport::OutboundCall;
use proto_pattern::{CallError, StatusCode};

use crate::std::client::MetadataExtender;

/// The headers of an [`OutboundCall`], for an interceptor. Bring the trait into scope to call it on the call.
pub trait OutboundCallHeaders {
    /// `true` when the call already carries a header named `key`.
    fn has_header(&self, key: &str) -> bool;

    /// The values of the headers named `key`, in the order they were added.
    fn get_header_values(&self, key: &str) -> Vec<&str>;

    /// Add the header `key: value` after the ones the call carries.
    ///
    /// # Errors
    ///
    /// [`CallError::Transport`] with `InvalidArgument` when `key` or `value` is not valid metadata: the caller's
    /// own input is refused, so a retry policy must not retry it.
    fn add_header(&mut self, key: &str, value: &str) -> Result<(), CallError>;
}

impl OutboundCallHeaders for OutboundCall<'_> {
    fn has_header(&self, key: &str) -> bool {
        !self.metadata.get_values(key).is_empty()
    }

    fn get_header_values(&self, key: &str) -> Vec<&str> {
        self.metadata.get_values(key)
    }

    fn add_header(&mut self, key: &str, value: &str) -> Result<(), CallError> {
        self.metadata = MetadataExtender::append(&self.metadata, key, value).map_err(|e| {
            CallError::Transport {
                path: self.path.into(),
                code: StatusCode::InvalidArgument,
                reason: e.to_string(),
            }
        })?;
        Ok(())
    }
}
