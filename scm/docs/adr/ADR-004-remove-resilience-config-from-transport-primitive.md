# ADR-004: Remove Resilience Config Primitive From the Transport Crate

**Status:** Proposed
**Date:** 2026-08-07
**See also:** [ADR-003](ADR-003-extract-retry-breaker-to-shared-crates.md) (this repo),
[edge-transport-http-egress ADR-005](https://github.com/sweengineeringlabs/edge-transport-http-egress/blob/dev/scm/docs/adr/ADR-005-retry-breaker-composition-moves-out-of-transport.md)
— same principle applied to the HTTP side, which violates it more directly (a real Cargo
dependency, not just a mirrored shape)
**Tracking:** [#14](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/14)

---

## Mandate

The base `transport` crate (`edge-transport-grpc-egress`) must have **zero knowledge** of
retry/breaker — not even the config *shape*. Remove `resilience: Option<ResilienceConfigResilienceValidator>`
from `GrpcChannelConfig`/`GrpcChannelConfigBuilder`, and delete
`ResilienceConfigResilienceValidator`/`ResilienceConfigBuilder` from `transport` entirely. The
concept moves fully into `edge-transport-grpc-egress-resilient`, which already does the actual
composition.

## Why

Unlike `edge-transport-http-egress` (see the sibling ADR above), this crate's base `transport` has
no Cargo dependency on `retry` or `breaker` — that separation was deliberate. But `GrpcChannelConfig`
(a `transport` primitive) still carries `resilience: Option<ResilienceConfigResilienceValidator>`,
and `ResilienceConfigResilienceValidator` (defined in `transport` itself,
`api/types/resilience_config_resilience_validator.rs`) is an 11-field struct hand-shaped to mirror
`GrpcRetryConfig` + `GrpcBreakerConfig` combined — specifically to let `transport` carry a
resilience config *without* a Cargo dependency on the crates that actually define that shape.

That's coupling without dependency-graph visibility, which is worse than a direct dependency, not
better: a real Cargo dependency gets a compiler error the moment `GrpcRetryConfig`'s or
`GrpcBreakerConfig`'s fields change shape. This mirrored struct gets silent drift instead —
`edge-transport-grpc-egress-resilient`'s `grpc_resilient_facade.rs` hand-maps all 11 fields
(`core/traits/grpc_resilient_facade.rs::create_resilient_transport_from_config`) with nothing
enforcing the two shapes stay in sync. Removing the Cargo edge didn't remove the coupling; it just
hid it from the compiler while leaving `transport`'s own primitive semantically shaped around a
concern (backoff multipliers, jitter factors, failure thresholds, half-open probe counts) that has
nothing to do with sending bytes over a gRPC channel.

## What changes

- `api/types/grpc_channel_config.rs`: `resilience: Option<ResilienceConfigResilienceValidator>`
  field removed from `GrpcChannelConfig`.
- `api/types/grpc_channel_config_builder.rs`: corresponding builder field/method removed.
- `api/types/resilience_config_resilience_validator.rs`, `resilience_config_builder.rs`: deleted
  from `transport` entirely.
- `edge-transport-grpc-egress-resilient`: gains its own resilience config type (same field shape,
  now the single source of truth — no mirror to keep in sync) and its own TOML
  `[channel.resilience]` loading, replacing what `transport`'s `GrpcChannelConfig` used to carry.
- `resilient`'s `grpc_resilient_facade.rs`: `create_resilient_transport_from_config` takes the
  resilience config as its own parameter (or loads it itself) instead of reading it off
  `GrpcChannelConfig`.

## Consequences

- **Breaking change** for any consumer configuring resilience via `GrpcChannelConfig`/
  `GrpcChannelConfigBuilder` directly — must switch to `edge-transport-grpc-egress-resilient`'s own
  config entry point.
- `transport`'s public API surface shrinks by one field and two types; `cargo build`/`test` for
  `transport` alone no longer touches anything resilience-shaped.
- No sequencing dependency on ADR-003's shared-crate extraction — this can land independently,
  before or after.
