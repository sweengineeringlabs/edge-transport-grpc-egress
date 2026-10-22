//! `PendingCall` -- the implementation of [`OutboundCall`]: a call that has not been sent.

use std::time::Duration;

use grpc_egress_transport::proto_pattern::{
    CallError, CallOptions, GetValuesRequest, GetValuesResponse, Metadata, MetadataView, StatusCode,
};
use grpc_egress_transport::{MetadataExtender, OutboundCall};

use crate::std::client::StdMetadataExtender;

/// The path, the timeout and the headers of a call about to be sent, which the interceptors of the chain may change.
pub(crate) struct PendingCall<'a> {
    path: &'a str,
    timeout: Option<Duration>,
    metadata: Metadata,
}

impl<'a> From<(&'a str, &CallOptions)> for PendingCall<'a> {
    /// The call to the path that the options describe.
    fn from((path, options): (&'a str, &CallOptions)) -> Self {
        Self {
            path,
            timeout: options.get_timeout(),
            metadata: options.get_metadata().clone(),
        }
    }
}

impl From<PendingCall<'_>> for CallOptions {
    /// The options the call carries once the interceptors have run.
    fn from(call: PendingCall<'_>) -> Self {
        CallOptions::create(call.timeout, call.metadata)
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
        !self
            .metadata
            .get_values(GetValuesRequest::new(key))
            .map(GetValuesResponse::into_values)
            .unwrap_or_default()
            .is_empty()
    }

    fn get_header_values(&self, key: &str) -> Vec<&str> {
        self.metadata
            .get_values(GetValuesRequest::new(key))
            .map(GetValuesResponse::into_values)
            .unwrap_or_default()
    }

    fn add_header(&mut self, key: &str, value: &str) -> Result<(), CallError> {
        self.metadata = StdMetadataExtender
            .append(&self.metadata, key, value)
            .map_err(|e| CallError::Transport {
                path: self.path.into(),
                code: StatusCode::InvalidArgument,
                reason: e.to_string(),
            })?;
        Ok(())
    }
}
