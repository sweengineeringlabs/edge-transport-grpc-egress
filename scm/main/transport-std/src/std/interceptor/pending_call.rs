//! `PendingCall` -- the implementation of [`OutboundCall`]: a call that has not been sent.

use std::time::Duration;

use grpc_egress_transport::OutboundCall;
use proto_pattern::{CallError, CallOptions, Metadata, StatusCode};

use crate::std::client::MetadataExtender;

/// The path, the timeout and the headers of a call about to be sent, which the interceptors of the chain may change.
pub(crate) struct PendingCall<'a> {
    path: &'a str,
    timeout: Option<Duration>,
    metadata: Metadata,
}

impl<'a> PendingCall<'a> {
    /// The call to `path` that `options` describe.
    pub(crate) fn create(path: &'a str, options: &CallOptions) -> Self {
        Self {
            path,
            timeout: options.get_timeout(),
            metadata: options.get_metadata().clone(),
        }
    }

    /// The options the call carries once the interceptors have run.
    pub(crate) fn into_options(self) -> CallOptions {
        CallOptions::create(self.timeout, self.metadata)
    }
}

impl OutboundCall for PendingCall<'_> {
    fn get_path(&self) -> &str {
        self.path
    }

    fn get_timeout(&self) -> Option<Duration> {
        self.timeout
    }

    fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }

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
