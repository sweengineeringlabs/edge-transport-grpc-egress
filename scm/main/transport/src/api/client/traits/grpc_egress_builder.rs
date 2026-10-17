//! `GrpcEgressBuilder` -- the options of a gRPC egress client, and the call that builds it.

use std::sync::Arc;
use std::time::Duration;

use proto_pattern::{Compression, RawTransport, TlsSettings, Transport};

use crate::{
    GrpcChannelConfig, GrpcChannelConfigError, GrpcEgressInterceptor, KeepAliveConfig,
    LoadbalancerSettings, TraceContextSource,
};

/// Collects the options of a client and builds it.
///
/// Every capability that can be configured is a `with_*` method here, so the API shows what a
/// client can do. A method takes `self` and returns `Self`: options are set in any order, the last
/// call for one option wins (except [`GrpcEgressBuilder::with_accept_compression`] and
/// [`GrpcEgressBuilder::with_interceptor`], which add), and nothing is sent or opened until
/// [`GrpcEgressBuilder::build`].
///
/// The client it builds is a [`Transport`] (a call over a `Method`) and a [`RawTransport`] (a call to a
/// `MethodPath`, with bytes for messages) at once, over one connection pool.
///
/// The implementation holds the state and does the work; a caller uses the options through this
/// trait without naming the implementation type.
pub trait GrpcEgressBuilder: Sized {
    /// Send requests to `endpoint` instead of the one the builder was created for.
    fn with_endpoint(self, endpoint: impl Into<String>) -> Self;

    /// Allow an endpoint whose scheme is plaintext (`http://`). Without it, `build` refuses one:
    /// TLS is required by default.
    fn with_plaintext_allowed(self) -> Self;

    /// End a call that has not completed after `timeout`, whatever its own timeout says: a call
    /// ends at the earlier of its own timeout and this one, whatever its shape. Without it the
    /// timeout is [`DEFAULT_REQUEST_TIMEOUT_SECS`](crate::DEFAULT_REQUEST_TIMEOUT_SECS).
    fn with_request_timeout(self, timeout: Duration) -> Self;

    /// Refuse a response message larger than `bytes`. Without it the limit is the engine's own, 4 MiB.
    fn with_max_decoding_message_size(self, bytes: usize) -> Self;

    /// Refuse to send a request message larger than `bytes`. Without it there is no limit.
    fn with_max_encoding_message_size(self, bytes: usize) -> Self;

    /// Compress the messages this client sends with `compression`; the server must accept it. Without
    /// it, messages are sent as they are.
    fn with_send_compression(self, compression: Compression) -> Self;

    /// Accept responses compressed with `compression`, and say so to the server (`grpc-accept-encoding`).
    /// May be called for more than one. A response compressed with an encoding that was not accepted is
    /// refused.
    fn with_accept_compression(self, compression: Compression) -> Self;

    /// Secure the connection with `tls`: the roots to trust (the platform's when none are given), the
    /// identity to present (mutual TLS) and the server name to verify. Required for an `https`
    /// endpoint and refused for an `http` one.
    fn with_tls(self, tls: TlsSettings) -> Self;

    /// Send HTTP/2 PING frames as `keep_alive` says and close the connection when one is not
    /// answered within its timeout, so that a peer that went away silently is noticed near that
    /// timeout and not at the next call's deadline. With `permit_without_calls` the pings are also
    /// sent while no call is in flight.
    fn with_keep_alive(self, keep_alive: KeepAliveConfig) -> Self;

    /// Spread calls over the backends of `settings` instead of sending them to one endpoint. The
    /// balanced client honours every other option the same way a single-endpoint client does: the
    /// message limits, compression, request timeout, TLS, keep-alive and interceptors.
    ///
    /// The backends carry their own URLs, so this cannot be combined with
    /// [`GrpcEgressBuilder::with_endpoint`]; `build` fails when both are set. The endpoint the
    /// factory was created with is not used.
    fn with_loadbalancer(self, settings: LoadbalancerSettings) -> Self;

    /// Run `interceptor` before and after every call. Interceptors run in the order they were added.
    fn with_interceptor(self, interceptor: Arc<dyn GrpcEgressInterceptor>) -> Self;

    /// Inject the trace context `source` describes into every call that does not carry one.
    fn with_trace_context(self, source: TraceContextSource) -> Self;

    /// Apply every setting of `config` in one call, as if each had been set with its own method;
    /// a method called afterwards overrides the setting it names. `max_message_bytes` sets both message
    /// limits, `compression` sets the send and the accepted compression, and the PEM files named by
    /// `mtls` are read by `build`.
    fn with_channel_config(self, config: &GrpcChannelConfig) -> Self;

    /// Validate every option and build the client. Nothing is sent: the connection opens at the first
    /// call.
    ///
    /// # Errors
    ///
    /// A [`GrpcChannelConfigError`] naming what is wrong, and for a conflict both options: a zero
    /// message limit or request timeout, mutual TLS with plaintext allowed, a load balancer with no
    /// backends or together with an endpoint, a plaintext endpoint without
    /// [`GrpcEgressBuilder::with_plaintext_allowed`], a TLS option on a plaintext endpoint, a TLS file
    /// that cannot be used, or an option the engine refuses.
    fn build(self) -> Result<impl Transport + RawTransport + use<Self>, GrpcChannelConfigError>;
}
