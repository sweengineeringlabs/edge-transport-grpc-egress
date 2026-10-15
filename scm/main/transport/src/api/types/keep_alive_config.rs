//! HTTP/2 keep-alive policy for gRPC channels.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// HTTP/2 keep-alive ping policy.
///
/// Defaults match gRPC's recommended client-side settings (10s
/// interval, 20s timeout).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KeepAliveConfig {
    /// Interval between PING frames.
    pub interval: Duration,
    /// Timeout for receiving a PING ACK.
    pub timeout: Duration,
    /// If `true`, send pings even when no streams are active.
    pub permit_without_calls: bool,
}
