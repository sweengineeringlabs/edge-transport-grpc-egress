//! HTTP/2 keep-alive policy for gRPC channels.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// HTTP/2 keep-alive ping policy.
///
/// Defaults match gRPC's recommended client-side settings (10s
/// interval, 20s timeout).
///
/// ```compile_fail,E0451
/// use grpc_egress_transport::KeepAliveConfig;
/// let _ = KeepAliveConfig { interval: std::time::Duration::ZERO, timeout: std::time::Duration::ZERO, permit_without_calls: false };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "KeepAliveConfigData")]
pub struct KeepAliveConfig {
    /// Interval between PING frames.
    interval: Duration,
    /// Timeout for receiving a PING ACK.
    timeout: Duration,
    /// If `true`, send pings even when no streams are active.
    permit_without_calls: bool,
}

#[derive(Deserialize)]
struct KeepAliveConfigData {
    interval: Duration,
    timeout: Duration,
    permit_without_calls: bool,
}
impl TryFrom<KeepAliveConfigData> for KeepAliveConfig {
    type Error = String;
    fn try_from(v: KeepAliveConfigData) -> Result<Self, String> {
        Self::try_new(v.interval, v.timeout, v.permit_without_calls)
    }
}

impl KeepAliveConfig {
    /// Construct an HTTP/2 keep-alive policy with positive intervals.
    pub fn try_new(
        interval: Duration,
        timeout: Duration,
        permit_without_calls: bool,
    ) -> Result<Self, String> {
        if interval.is_zero() {
            return Err("interval must be positive".into());
        }
        if timeout.is_zero() {
            return Err("timeout must be positive".into());
        }
        Ok(Self {
            interval,
            timeout,
            permit_without_calls,
        })
    }
    /// Ping interval.
    pub fn interval(&self) -> Duration {
        self.interval
    }
    /// Ping acknowledgement timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
    /// Whether to ping with no active calls.
    pub fn permit_without_calls(&self) -> bool {
        self.permit_without_calls
    }
}

impl Default for KeepAliveConfig {
    /// gRPC's recommended client-side settings: ping every 10 s, give up after 20 s, only while
    /// a call is in flight.
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(10),
            timeout: Duration::from_secs(20),
            permit_without_calls: false,
        }
    }
}
