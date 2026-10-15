//! `GrpcEgressBuilder` -- the options of a gRPC egress client, and the call that builds it.

use std::sync::Arc;
use std::time::Duration;

use crate::api::{
    CompressionMode, GrpcChannelConfig, GrpcChannelConfigError, GrpcEgress, GrpcEgressInterceptor,
    TraceContextSource,
};

/// Collects the options of a client and builds it.
///
/// Every capability that can be configured is a `with_*` method here, so the API shows what a
/// client can do. A method takes `self` and returns `Self`: options are set in any order, the last
/// call for one option wins, and nothing is sent or opened until [`GrpcEgressBuilder::build`].
///
/// The implementation holds the state and does the work; a caller uses the options through this
/// trait without naming the implementation type.
pub trait GrpcEgressBuilder: Sized {
    /// Send requests to `endpoint` instead of the one the builder was created for.
    fn with_endpoint(self, endpoint: impl Into<String>) -> Self;

    /// Allow an endpoint whose scheme is plaintext (`http://`). Without it, `build` refuses one:
    /// TLS is required by default.
    fn with_plaintext_allowed(self) -> Self;

    /// End a call that has not completed after `timeout`, whatever its own deadline says: a call
    /// ends at the earlier of its deadline and this timeout. Without it the timeout is
    /// [`DEFAULT_REQUEST_TIMEOUT_SECS`](crate::api::DEFAULT_REQUEST_TIMEOUT_SECS).
    fn with_request_timeout(self, timeout: Duration) -> Self;

    /// Refuse a response message larger than `bytes`. Without it the cap is
    /// [`DEFAULT_MAX_MESSAGE_BYTES`](crate::api::DEFAULT_MAX_MESSAGE_BYTES).
    fn with_max_message_bytes(self, bytes: usize) -> Self;

    /// Compress outgoing messages with `mode`.
    fn with_compression(self, mode: CompressionMode) -> Self;

    /// Run `interceptor` before and after every call. Interceptors run in the order they were added.
    fn with_interceptor(self, interceptor: Arc<dyn GrpcEgressInterceptor>) -> Self;

    /// Inject the trace context `source` describes into every call that does not carry one.
    fn with_trace_context(self, source: TraceContextSource) -> Self;

    /// Apply every setting of `config` in one call, as if each had been set with its own method;
    /// a method called afterwards overrides the setting it names.
    fn with_channel_config(self, config: &GrpcChannelConfig) -> Self;

    /// Check the options and build the client. Nothing has been sent when this returns.
    ///
    /// # Errors
    ///
    /// A [`GrpcChannelConfigError`] naming what is wrong: a plaintext endpoint without
    /// [`GrpcEgressBuilder::with_plaintext_allowed`], or an option the client cannot apply.
    fn build(self) -> Result<impl GrpcEgress + use<Self>, GrpcChannelConfigError>;
}
