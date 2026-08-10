# ADR-004: Remove Resilience Config Primitive From the Transport Crate

**Status:** Accepted (amended 2026-08-10 — see Amendment: one-composer / decorator)
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

---

## Amendment (2026-08-10): one composer — retire the composing facade, expose a decorator

The original decision above removes the resilience *config* from `transport` but **keeps**
`edge-transport-grpc-egress-resilient`'s `create_resilient_transport_from_config` — the facade that
builds the base gRPC client *and* wraps it (`base → GrpcRetryClient → GrpcBreakerClient`). This
amendment extends ADR-004: that facade is a **composing library** (a library assembling a finished
client), which the wider `edge` architecture forbids.

**Principle (confirmed with the repo owner; established on the HTTP side — edge-bootstrap ADR-008,
edge-transport-http-egress#28/#29/#37/#38):** there is exactly **one composer**, `edge-bootstrap`'s
`RuntimeBuilder`. Libraries expose *primitives* — a bare transport, and a *decorator* — never a
composed result. The HTTP `resilient` crate models this: it exposes `DefaultResilientLayers.apply_defaults`,
a decorator applied to an already-built client by the composition root. The gRPC `resilient` crate must
do the same.

### Additional changes (supersede "keep the facade")

- `edge-transport-grpc-egress-resilient` exposes a public **decorator**: wrap an already-built
  `Arc<dyn GrpcEgress>` in retry + breaker — e.g.
  `apply_resilience(inner: Arc<dyn GrpcEgress>, cfg: ResilienceConfig) -> Arc<dyn GrpcEgress>`.
  `GrpcRetryClient`/`GrpcBreakerClient` (today `pub(crate)`) and their configs become public, with
  SWE-default values.
- `create_resilient_transport_from_config` (base-building facade) is **removed**. The composition root
  builds the base via `transport::create_transport_from_config` and applies the decorator itself.
- The resilience config type still lands in `resilient` (Layer 1 above), now consumed by the decorator
  rather than the facade.

### Consumer

`edge-bootstrap`'s `RuntimeBuilder` composes gRPC egress = bare client + resilient decorator, alongside
its HTTP equivalent — one composer, both protocols. Tracked: edge-transport-grpc-egress#14 (Layer 2),
edge-bootstrap#39. This also makes gRPC egress resilient in bootstrap for the first time (today it calls
the bare path and silently gets no retry/breaker).
