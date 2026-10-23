# ADR-012: Adapt egress interceptors to the Pattern call lifecycle

**Status:** Implemented on `dev` in `7a5ab54`; issue [#61](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/61) is closed
**Tracking:** [#61](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/61)
**Governing contract:** [proto-pattern v0.12.0 `TransportInterceptor`](https://github.com/sweengineeringlabs/proto-pattern/blob/v0.12.0/main/src/traits/transport_interceptor.rs)
**See also:** [ADR-007](ADR-007-decorators-wrap-a-built-client.md), [ADR-009](ADR-009-the-egress-api-on-the-contract.md)

## Context

Pattern v0.12.0 defines a pre-call hook and a typed/raw lifecycle event contract. Egress already exposes runtime registration through `Arc<dyn GrpcEgressInterceptor>`. That API also has a fallible `after_call` hook, which can reject a successful unary or client-streaming reply. Pattern's `on_event` is generic over metadata and observational, so it cannot replace the egress trait object or its reply policy without changing behavior and breaking consumers.

## Decision

`InterceptedTransport` creates a per-call adapter that implements Pattern's `TransportInterceptor`. It supplies `CallIdentity` and `CallKind` for every typed and raw operation, delegates pre-call option changes to the existing runtime chain in registration order, and reports Pattern lifecycle events for stream open, message reads, completion, errors and early drops. The chain receives the original Pattern `CallEvent` type with tonic's concrete `Metadata`; metadata conversion for callbacks is best-effort and cannot change the RPC outcome.

`GrpcEgressInterceptor` remains the runtime registration boundary. Its default object-safe `on_event` method forwards lifecycle observations without changing existing implementors. Its fallible `after_call` remains an egress-specific reply policy and still runs only for successful unary and client-streaming calls. If that policy rejects a reply, egress reports `Failed` and returns the same error. Pattern events themselves remain observational.

The builder facade, endpoint and backend rules, `with_channel_config` overlays, and documented repeat semantics stay stable. Portable endpoint, TLS, timeout, message limit and compression options continue through Pattern's `TransportConfiguration` as implemented by the service engine. Egress retains the values it must apply once per load-balanced backend, as well as file-based TLS resolution and channel-config precedence. HTTP/2 keep-alive remains a service-engine option. Retry, breaker, authentication, trace policy, load-balancing policy and resilience composition remain with their existing owners; ADR-007 decorator order is unchanged.

## Consequences

Existing `with_interceptor` and `with_trace_context` call sites continue to compile and preserve their current behavior. Interceptors can additionally observe `CallEvent` without defining a second lifecycle event model. Typed and raw stream tests against the real tonic peer prove headers, messages, terminal metadata and cancellation behavior. No test is removed; the mutation manifest covers each event boundary. A future removal of fallible `after_call` or runtime registration would require a separate breaking-change decision and migration guide.
