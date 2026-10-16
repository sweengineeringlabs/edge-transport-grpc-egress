# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed (breaking)
- `GrpcResilientFacade::create_resilient_transport_from_config` (built a base transport client
  *and* composed retry+breaker onto it — a second composer) is removed. Replaced by
  `GrpcResilientFacade::apply_resilience(inner, config)`, a pure decorator that wraps an
  already-built bare `GrpcEgress` — the composition root builds the base itself, per the
  one-composer model (ADR-004 amendment, edge-bootstrap ADR-008/010).
- `ResilienceConfig` is no longer a newtype wrapper around
  `edge_transport_grpc_egress::ResilienceConfigResilienceValidator` (that type is gone from
  `transport`). It's now this crate's own owned struct — same field shape, but this crate is the
  sole source of truth, with its own `Default`, validation, and TOML loading
  (`ConfigSection::section_name() == "grpc_resilience"`).
- `ResilientTransportError::ChannelConfig` is removed (this crate no longer builds a base client,
  so there's no channel-config error to surface). Added `ResilientTransportError::Breaker`,
  wrapping a breaker-decorator failure.

### Added
- `ResilienceConfig::from_config(toml_text)` — parse and validate a `[grpc_resilience]`-shaped TOML
  section, mirroring `GrpcRetryConfig`/`GrpcBreakerConfig`'s own `from_config`.
- Initial resilient gRPC transport assembly: `TonicGrpcClient` + retry + circuit-breaker stack.
