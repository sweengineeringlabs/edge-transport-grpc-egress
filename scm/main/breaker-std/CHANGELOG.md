# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed (breaking)
- The circuit-state machine (`BreakerTransition`, `BreakerState`, `BreakerNode`, `Admission`,
  `Outcome`, `AdmitRequest`/`AdmitResponse`, `RecordOutcomeRequest`/`RecordOutcomeResponse`,
  `FailureClassifier`, `ClassifyRequest`/`ClassifyResponse`, `Error`/`BreakerDomainError`) moved to
  the shared crate `edge-transport-breaker-policy`/`edge-transport-breaker` — this repo's
  `DefaultBreakerTransition` was the canonical implementation chosen (see
  [edge-transport-breaker CHANGELOG](https://github.com/sweengineeringlabs/edge-transport-breaker/blob/main/scm/CHANGELOG.md)
  for the design rationale). This crate re-exports the same names from the shared crate, so most
  call sites are unaffected; `GrpcBreakerConfig` gained `impl From<GrpcBreakerConfig> for
  edge_transport_breaker_policy::BreakerConfig` since `AdmitRequest`/`RecordOutcomeRequest` now
  take the shared config type.
- `Error`'s Display messages no longer embed `edge_transport_grpc_egress_breaker` — they carry a
  protocol-agnostic `"breaker policy:"` prefix instead, since the type is shared across protocols.

### Added
- Initial gRPC circuit breaker decorator: three-state machine (Closed/Open/HalfOpen) with configurable failure threshold and cool-down.
