//! Interface counterpart for `core/resilience/resilient_grpc_client.rs`.
//!
//! [`ResilientGrpcClientPort`] is the api/ contract implemented by the
//! concrete `core::resilience::ResilientGrpcClient`.  Callers retrieve it
//! from the SAF layer as `Arc<dyn GrpcEgress>` — this trait is the
//! documentation anchor and extension point.

use std::time::Duration;

use crate::api::error::GrpcEgressError;
use crate::api::traits::GrpcEgress;
use crate::api::{
    CircuitStateRequest, CircuitStateResponse, ConsecutiveFailuresRequest,
    ConsecutiveFailuresResponse, GrpcChannelConfig, GrpcChannelConfigBuilder, KeepAliveConfig,
    LastErrorRequest, LastErrorResponse, MtlsConfig,
};

/// Extension contract for a gRPC client that adds resilience (retry + circuit breaker).
///
/// The concrete implementation lives in `core/`; consumers interact with the
/// type-erased `Arc<dyn GrpcEgress>` surface returned by the SAF factory
/// functions.
pub trait ResilientGrpcClientPort: GrpcEgress + Send + Sync {
    /// Return the current circuit-breaker state label for observability.
    ///
    /// Implementations must return one of: `"Closed"`, `"Open"`, `"HalfOpen"`.
    fn circuit_state(
        &self,
        req: CircuitStateRequest,
    ) -> Result<CircuitStateResponse, GrpcEgressError>;

    /// Return the count of consecutive post-retry failures tracked by the
    /// circuit breaker since it last closed.
    fn consecutive_failures(
        &self,
        req: ConsecutiveFailuresRequest,
    ) -> Result<ConsecutiveFailuresResponse, GrpcEgressError>;

    /// Expose the last transport error seen by the resilience layer, if any.
    ///
    /// `error` is `None` when no failure has been recorded (circuit is
    /// `Closed` and no retry storms have fired).
    fn last_error(&self, req: LastErrorRequest) -> Result<LastErrorResponse, GrpcEgressError>;

    /// Report the endpoint a resilient client was configured against —
    /// gives [`GrpcChannelConfig`] a genuine role in this trait's
    /// signature set. `Self: Sized` keeps this trait dyn-compatible for
    /// `Box<dyn Trait>`.
    fn describe_channel_config(config: &GrpcChannelConfig) -> &str
    where
        Self: Sized,
    {
        &config.endpoint
    }

    /// Start a fluent [`GrpcChannelConfigBuilder`] for programmatic channel
    /// construction — gives it a genuine role in this trait's signature
    /// set, not just an impl-site helper. `Self: Sized` keeps this trait
    /// dyn-compatible for `Box<dyn Trait>`.
    fn default_channel_config_builder() -> GrpcChannelConfigBuilder
    where
        Self: Sized,
    {
        GrpcChannelConfigBuilder::new()
    }

    /// Extract the PING interval from a keep-alive policy — gives
    /// [`KeepAliveConfig`] a genuine role in this trait's signature set,
    /// not just an internal channel-config field. `Self: Sized` keeps this
    /// trait dyn-compatible for `Box<dyn Trait>`.
    fn describe_keep_alive(cfg: KeepAliveConfig) -> Duration
    where
        Self: Sized,
    {
        cfg.interval
    }

    /// Report the client certificate path from an mTLS identity — gives
    /// [`MtlsConfig`] a genuine role in this trait's signature set, not
    /// just an internal channel-config field. `Self: Sized` keeps this
    /// trait dyn-compatible for `Box<dyn Trait>`.
    fn describe_mtls(cfg: &MtlsConfig) -> &str
    where
        Self: Sized,
    {
        &cfg.cert_pem_path
    }
}
