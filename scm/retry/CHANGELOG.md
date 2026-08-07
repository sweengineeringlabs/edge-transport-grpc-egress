# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed (breaking)
- The backoff/jitter core (`BackoffScheduler`'s `next_backoff`/`rate_limit_backoff`,
  `DefaultJitterRng`, `JitterRng`, `NextUnitRequest`/`NextUnitResponse`,
  `Error`/`RetryError`) moved to the shared crate `edge-transport-retry-policy`/
  `edge-transport-retry` — this repo's implementation was the canonical one chosen (it was the
  only side with clean, standalone, protocol-agnostic files; see
  [edge-transport-retry CHANGELOG](https://github.com/sweengineeringlabs/edge-transport-retry/blob/main/scm/CHANGELOG.md)).
  This crate re-exports the same names from the shared crate, so most call sites are unaffected;
  `GrpcRetryConfig` gained `impl BackoffPolicy`/`impl RateLimitBackoffPolicy` since the shared
  scheduler is now generic over those traits instead of the concrete `GrpcRetryConfig`.
  `BackoffTrack`/`BackoffScheduleRequest`/`DefaultBackoffScheduler` (track-selection
  orchestration, which classifies gRPC-specific error types) stay local — protocol-specific by
  nature, not moved.
- `Error`'s Display messages no longer embed `edge_transport_grpc_egress_retry` — they carry a
  protocol-agnostic `"retry policy:"` prefix instead, since the type is shared across protocols.

### Added
- Initial gRPC retry decorator: exponential backoff with jitter, deadline-bounded budget, status-code-aware retry policy.
