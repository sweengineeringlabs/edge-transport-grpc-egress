//! Discriminates the cause of a `RESOURCE_EXHAUSTED` (gRPC 8) error.
//!
//! The same status code covers three situations that require different
//! retry strategies:
//!
//! | Context     | Cause                        | Correct response          |
//! |-------------|------------------------------|---------------------------|
//! | `Capacity`  | Server OOM / CPU saturation  | Retry standard track      |
//! | `RateLimit` | API rate-limit window full   | Retry rate-limit track    |
//! | `HardQuota` | Billing quota exhausted      | Do not retry; escalate    |
//!
//! Classification inspects the `grpc-message` string for well-known
//! keywords. `Capacity` is the safe default — it triggers a retry,
//! which is always better than silently dropping the request.

/// Discriminates the cause of a `RESOURCE_EXHAUSTED` (gRPC 8) error.
///
/// Classification inspects the `grpc-message` string for well-known
/// keywords. `Capacity` is the safe default — it triggers a retry,
/// which is always better than silently dropping the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceExhaustedContext {
    /// Server capacity or OOM — may clear on retry after backoff.
    Capacity,
    /// API rate limit — the request window is full; retry after reset.
    RateLimit,
    /// Billing / quota hard cap — retry will not help.
    HardQuota,
}

impl ResourceExhaustedContext {
    /// Classify a `RESOURCE_EXHAUSTED` grpc-message into a context.
    pub fn classify(message: &str) -> Self {
        let msg = message.to_ascii_lowercase();
        if msg.contains("quota") || msg.contains("billing") || msg.contains("plan limit") {
            ResourceExhaustedContext::HardQuota
        } else if msg.contains("rate")
            || msg.contains("too many requests")
            || msg.contains("throttl")
        {
            ResourceExhaustedContext::RateLimit
        } else {
            ResourceExhaustedContext::Capacity
        }
    }
}
