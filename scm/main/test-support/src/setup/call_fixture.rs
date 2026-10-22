//! [`CallFixture`] -- the path and the request of a raw call, written the short way.

use std::time::Duration;

use proto_pattern::{CallOptions, CallRequest, Metadata, MethodPath};

/// Builds the path and the request of a raw call for a test. A test that is about a path or a request that is not
/// valid builds it with the contract's own constructors instead.
pub struct CallFixture;

impl CallFixture {
    /// The path of `method`, written with or without the leading `/` (`pkg.Service/Method`).
    ///
    /// # Panics
    ///
    /// When `method` is not a `service/method` path: a test that names one that is not has a typo.
    pub fn path(method: &str) -> MethodPath {
        MethodPath::create(format!("/{}", method.trim_start_matches('/')))
            .unwrap_or_else(|e| panic!("the test names a method path that is not valid: {e}"))
    }

    /// A request that carries `body` and ends after `timeout`, with no headers.
    pub fn request(body: &[u8], timeout: Duration) -> CallRequest<Vec<u8>> {
        Self::request_with(body, timeout, Metadata::default())
    }

    /// A request that carries `body` and `metadata` and ends after `timeout`.
    pub fn request_with(
        body: &[u8],
        timeout: Duration,
        metadata: Metadata,
    ) -> CallRequest<Vec<u8>> {
        CallRequest::new(body.to_vec(), CallOptions::create(Some(timeout), metadata))
    }
}
