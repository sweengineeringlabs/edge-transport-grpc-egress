//! `impl` block for [`RetryDecision`]. The type *declaration* lives in
//! `api/`.

use std::time::Duration;

use edge_transport_grpc_egress::{GrpcEgressError, GrpcStatusCode};

use crate::api::{ResourceExhaustedContext, RetryDecision};

impl RetryDecision {
    /// True if the decision indicates the call should be re-issued.
    pub fn should_retry(self) -> bool {
        matches!(self, RetryDecision::Retry | RetryDecision::RetryRateLimit)
    }

    /// True if the decision is the rate-limit variant.
    pub fn is_rate_limit(self) -> bool {
        matches!(self, RetryDecision::RetryRateLimit)
    }

    /// Classify an outbound result into a retry decision.
    ///
    /// Mapping table (for non-`Ok` outcomes):
    ///
    /// | Variant                                            | Decision          |
    /// |----------------------------------------------------|-------------------|
    /// | `Status(Unavailable, _)` / `Unavailable(_)`        | `Retry`           |
    /// | `Status(ResourceExhausted, _)` — RateLimit msg     | `RetryRateLimit`  |
    /// | `Status(ResourceExhausted, _)` — Capacity msg      | `Retry`           |
    /// | `Status(ResourceExhausted, _)` — HardQuota msg     | `Terminal`        |
    /// | `Status(Unauthenticated, _)`                       | `Terminal`        |
    /// | `Status(PermissionDenied, _)`                      | `Terminal`        |
    /// | `Status(DeadlineExceeded, _)` / `Timeout(_)`       | `Terminal`        |
    /// | `Status(Internal, _)` / `Internal(_)`              | `Terminal`        |
    /// | `ConnectionFailed(_)`                              | `Retry`           |
    /// | `Cancelled(_)` / `Status(Cancelled, _)`            | `Terminal`        |
    /// | other `Status(_, _)`                               | `Terminal`        |
    ///
    /// `ConnectionFailed` is treated as `Retry` because it's a
    /// transport-level transient (DNS hiccup, TCP RST during a rolling
    /// deploy) matching canonical `Unavailable` gRPC semantics.
    pub fn classify<T>(result: &Result<T, GrpcEgressError>) -> Self {
        let err = match result {
            Ok(_) => return RetryDecision::Success,
            Err(e) => e,
        };
        match err {
            GrpcEgressError::Status(code, msg) => match code {
                GrpcStatusCode::Unavailable => RetryDecision::Retry,
                GrpcStatusCode::ResourceExhausted => {
                    match ResourceExhaustedContext::classify(msg) {
                        ResourceExhaustedContext::HardQuota => RetryDecision::Terminal,
                        ResourceExhaustedContext::RateLimit => RetryDecision::RetryRateLimit,
                        ResourceExhaustedContext::Capacity => RetryDecision::Retry,
                    }
                }
                // Explicit non-retryable variants — listed here so
                // adding a new variant on `GrpcStatusCode` surfaces
                // as a missing arm, not a silent default.
                GrpcStatusCode::Unauthenticated
                | GrpcStatusCode::PermissionDenied
                | GrpcStatusCode::DeadlineExceeded
                | GrpcStatusCode::Internal
                | GrpcStatusCode::Cancelled
                | GrpcStatusCode::Ok
                | GrpcStatusCode::Unknown
                | GrpcStatusCode::InvalidArgument
                | GrpcStatusCode::NotFound
                | GrpcStatusCode::AlreadyExists
                | GrpcStatusCode::FailedPrecondition
                | GrpcStatusCode::Aborted
                | GrpcStatusCode::OutOfRange
                | GrpcStatusCode::Unimplemented
                | GrpcStatusCode::DataLoss => RetryDecision::Terminal,
            },
            GrpcEgressError::ConnectionFailed(_) => RetryDecision::Retry,
            GrpcEgressError::Unavailable(_) => RetryDecision::Retry,
            GrpcEgressError::Timeout(_)
            | GrpcEgressError::Internal(_)
            | GrpcEgressError::Cancelled(_) => RetryDecision::Terminal,
        }
    }

    /// Extract a `Retry-After` hint embedded in a gRPC error message.
    ///
    /// The transport embeds the HTTP `Retry-After` (or
    /// `x-ratelimit-reset-requests`) header value as `[retry-after=Ns]`
    /// at the end of the `grpc-message` when it receives a
    /// `RESOURCE_EXHAUSTED` response.  This lets the retry loop honour
    /// the upstream reset window for rate-limit errors without requiring
    /// a new error variant or extra fields.
    ///
    /// Returns `None` when no hint is present or when the value cannot
    /// be parsed as a whole number of seconds.
    pub fn parse_retry_after_hint(message: &str) -> Option<Duration> {
        let tag = "[retry-after=";
        let start = message.find(tag)? + tag.len();
        let rest = &message[start..];
        let end = rest.find('s')?;
        let secs: u64 = rest[..end].parse().ok()?;
        Some(Duration::from_secs(secs))
    }
}
