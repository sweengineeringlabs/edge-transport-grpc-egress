# ADR-011: Validated immutable configuration values

**Status:** Accepted for issue #62
**Tracking:** [#62](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/62)
**Governing reference:** [Agent Deployment Architecture](https://github.com/sweengineeringlabs/llm-agent-deployment-spec/blob/6a71e2ddac31579ca314919e3e54ca5c45d054cf/docs/3-design/architecture.md)
**See also:** [ADR-006](ADR-006-domain-first.md), [ADR-010](ADR-010-programming-over-the-interface.md)

## Context

Public configuration structs with public fields or unchecked derived deserialization let callers create completed values that violate the policy enforced later by a builder or provider. That weakens the domain boundary: every consumer must remember to validate, and invalid values can travel through the API before rejection. The pinned deployment architecture requires immutable value objects with private state, value semantics and explicit construction, while caller-owned values must remain constructible by callers.

## Decision

Configuration values keep private fields and provide read-only accessors. Their public constructors and custom deserialization enforce the same invariants. Existing builder APIs remain available where they are the natural multi-option construction path. Config values composed from other policies accept those validated policies directly rather than duplicate unchecked scalar fields.

The rule applies to retry, breaker, resilience, bearer, channel, keep-alive, mTLS, backend-target and load-balancer settings. It does not turn construction-phase `Build*Request` bundles into exposure DTOs, nor does it change contract types owned by `proto-pattern` or `proto-svc`.

`GrpcChannelConfig::default()` is retained as a builder seed, not a valid completed endpoint configuration. The public constructor, deserializer and channel builder all reject an empty endpoint. Retry multipliers retain their `f64` representation; validation rejects non-finite values, which makes the config's `Eq` implementation reflexive.

Bearer secret variants are replaced with `hs256` and `rs256` validated constructors and matching read-only accessors. Serialization retains the existing tagged shape to preserve configuration-file compatibility, while deserialization rejects empty key bytes.

Breaker configuration adds `enabled`, defaulting to true. A disabled breaker transparently forwards calls and neither admits them into breaker state nor records outcomes. Retry and breaker composition remains generic and statically dispatched.

## Consequences

Direct struct literals, public field reads, and direct `BearerSecret` enum variant construction are source-breaking. Callers migrate to constructors/builders and accessors; the workspace changelog records the change. Invalid serialized policy values fail during deserialization instead of later in composition. Compile-fail doctests and `scripts/check_layout.py` enforce the encapsulation boundary.

No change to upstream APIs was required. The workspace remains pinned to `proto-pattern` v0.12.0 and `proto-svc` v0.4.2.
