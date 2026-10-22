//! [`OutboundCallFixture`] -- a call that has not been sent, for a test that runs an interceptor on its own.

use proto_pattern::{
    CallError, GetBinaryEntriesRequest, GetEntriesRequest, GetValuesRequest, GetValuesResponse,
    Metadata, MetadataBuilder, MetadataView, StatusCode,
};
use std::time::Duration;

use grpc_egress_transport::OutboundCall;

/// An implementation of [`OutboundCall`] for tests, written without the library's own: a test that runs an interceptor
/// on its own needs a call to hand it, and one that shares code with the code under test would prove nothing.
pub struct OutboundCallFixture {
    path: String,
    timeout: Option<Duration>,
    metadata: Metadata,
}

impl OutboundCallFixture {
    /// A call to `path` with no timeout and no headers.
    pub fn create(path: &str) -> Self {
        Self {
            path: path.to_string(),
            timeout: None,
            metadata: Metadata::default(),
        }
    }

    /// The same call with `timeout`.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// The same call carrying `metadata`.
    #[must_use]
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// The headers the call carries now.
    pub fn get_metadata(&self) -> &Metadata {
        &self.metadata
    }
}

impl OutboundCall for OutboundCallFixture {
    fn get_path(&self) -> &str {
        &self.path
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
        let mut builder = MetadataBuilder::create();
        for entry in self
            .metadata
            .get_entries(GetEntriesRequest::new())
            .map_err(|e| CallError::Transport {
                path: self.path.as_str().into(),
                code: StatusCode::InvalidArgument,
                reason: e.to_string(),
            })?
            .into_entries()
        {
            let _ = builder.append(entry.get_key(), entry.get_value());
        }
        for entry in
            MetadataView::get_binary_entries(&self.metadata, GetBinaryEntriesRequest::new())
                .map_err(|e| CallError::Transport {
                    path: self.path.as_str().into(),
                    code: StatusCode::InvalidArgument,
                    reason: e.to_string(),
                })?
                .into_entries()
                .flatten()
        {
            let _ = builder.append_binary(entry.get_key(), entry.get_value());
        }
        builder
            .append(key, value)
            .map_err(|e| CallError::Transport {
                path: self.path.as_str().into(),
                code: StatusCode::InvalidArgument,
                reason: e.to_string(),
            })?;
        self.metadata = builder.build();
        Ok(())
    }
}
