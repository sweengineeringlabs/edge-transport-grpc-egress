# Changelog

All notable changes to `edge-transport-grpc-egress-transport` are documented here.

## Unreleased

- **Breaking**: removed `GrpcChannelConfig.resilience` and every type that existed solely to serve
  it — `ResilienceConfigResilienceValidator`, `ResilienceConfigBuilder`, the `Validator`/
  `ResilienceValidator` traits, `ValidationRequest`, `ConfigValidationRequest`, `ValidatorFactory`,
  `ResilienceValidatorFactory`, and `GrpcChannelConfig::with_resilience`/
  `GrpcChannelConfigBuilder::resilience`. `transport` now carries zero resilience-shaped knowledge
  (ADR-004 Layer 1) — resilience composition lives entirely in `resilient` and the composition root.
- **Breaking**: removed `GrpcChannelConfigError::Config` — it became dead the moment resilience
  validation left this crate; nothing in production constructed it anymore.
- Fixed two stale doc references left over from earlier refactors: `api/config/mod.rs` pointed at
  a nonexistent `api/vo/` path, and a doc comment in `tonic_grpc_egress_protocol.rs` linked to the
  now-deleted `core::types::resilience` module.
- Initial release of the gRPC outbound transport crate.
