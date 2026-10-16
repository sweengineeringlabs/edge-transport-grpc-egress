# ADR-007: Decorators wrap a built client; one factory per theme

**Status:** Proposed, implemented on `dev`; the owner's acceptance is requested (issue 16 task 13 asks for the ruling to be recorded before the decorator options, issue 17 acceptance criterion 4 for an accepted ADR)
**Date:** 2026-10-16
**Tracking:** [#16](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/16) tasks 7, 13, 14, 16; [#17](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/17) tasks 8, 20
**See also:** [ADR-004](ADR-004-remove-resilience-config-from-transport-primitive.md) (the transport knows nothing of retry and breaker), [ADR-006](ADR-006-domain-first.md) (domain crate and implementation crate)

---

## Context

Issue 16 asked where `with_retry`, `with_breaker` and `with_bearer_auth` live, given ADR-004. Three
options: (a) the owner's composer in `edge-bootstrap` chains the decorators; (b) a separate composing
crate above the transport; (c) each decorator crate has its own factory that wraps a built client.
Issue 17 asked for one factory per theme in the shape of edge-llm#226: a trait in the domain crate, one
`Std*Factory` in `saf/`, no implementation in `saf/`, no `Arc<dyn XFactory>`.

## Decision

**Placement: option (c).** The transport builder takes no decorator. Each decorator crate has a
factory that wraps any `GrpcEgress`:

| Theme | Domain trait | Implementation | Product |
|---|---|---|---|
| retry | `GrpcRetryFactory` | `StdGrpcRetryFactory::new(config)` | `impl GrpcEgress` |
| breaker | `GrpcBreakerFactory` | `StdGrpcBreakerFactory::new(config)` | `impl GrpcEgress + BreakerObservable` (the state stays observable) |
| resilient | `GrpcResilientFactory` | `StdGrpcResilientFactory::new(config)` | the two above composed |
| bearer auth | the transport's `GrpcEgressInterceptorFactory` | `StdBearerInterceptorFactory::new(config)` | an interceptor, not a wrapper: it signs inside the transport's interceptor chain |

The policy is bound in the factory's constructor; `build(req)` takes the request value
(`Build*Request<T> { inner }`) and returns the product, never a trait object. The client types
(`GrpcRetryClient`, `GrpcBreakerClient`, the bearer interceptor) are `pub(crate)`.

**Order, fixed and documented:** interceptors innermost (inside the transport client), then retry,
then the breaker outside. A tripped breaker short-circuits before retry runs, so an open circuit is not
amplified by retries. `StdGrpcResilientFactory` builds exactly this stack; the composition-root example
(scenario 6) and `resilient_stack_int_test.rs` pin it.

**Interceptor order (issue 16 task 7):** `before_call` and `after_call` both run in registration order
and stop at the first error. `after_call` is not reversed. The owner's question, keep or reverse,
is answered by keeping: reversing would change behaviour that callers may already rely on, and the
scenario test pins the current order (`InterceptorScenario`, mutant R14 of
`scripts/mutation_proof_composition_root.py`).

**Prost and feature-dependent options:** the option is always present on the builder; without the Cargo
feature, `build()` returns `DisabledFeatureOptions`, listing every disabled option at once. A drift-guard
test reads `[features]` from `Cargo.toml` and fails when a feature has no builder option.

**Removed as scaffolding:** every `Processor`, `Validator`, `ConfigBuilderProvider`, `RetryDecorator`,
`RetryInspector`, `BackoffScheduler` and `BreakerDecorator` trait with its `Default*` implementation,
request and response DTOs, `*Svc` marker types, facades and the old `ProcessorFactory`,
`ValidatorFactory`, `ConfigBuilderProviderFactory` and `GrpcEgressFactory`. The reason is checkable:
the retry loop and the breaker loop call `edge-transport-retry` and `edge-transport-breaker` directly, and
nothing reachable from a client used any of them. Validation of a policy is a method of the
implementation's `*ConfigProvider` extension trait and is run by the factory's `build`.

## Consequences

- Breaking: the old factory functions, the facades (`GrpcRetryFacade::wrap_retry`,
  `GrpcBreakerFacade::wrap_breaker`, `GrpcResilientFacade::apply_resilience`) and the public client types
  are gone. A consumer builds through the factories, as `scm/main/examples/composition-root` shows.
- A consumer that wants the state of the breaker gets it from the product, without naming a type.
- The domain crates `retry`, `breaker` and `resilient` now depend on `grpc-egress-transport` for
  `GrpcEgress`; the implementation crates depend on their domain crate.

## Rejected

(a) leaves every consumer to rebuild the order by hand, and (b) adds a fourth crate for what the
decorator crates already do. Keeping the facades as one-line calls to the factories was rejected because
issue 16 task 16 asks that no factory keeps a second copy of the construction path.
