# edge-transport-grpc-egress

**Audience:** Consumers composing outbound gRPC clients and maintainers of this Rust workspace.

## WHAT

Outbound gRPC client for swe-edge: a client over the `proto-svc` engine that is a `proto-pattern` `Transport`
and `RawTransport`, with retry and circuit-breaker decorators and a bearer-auth interceptor as separate crates.

Key capabilities:

- **`Transport`** and **`RawTransport`** (`proto-pattern`): the client the builder returns is both: a call over a `Method`, and a call to a `MethodPath` with bytes for messages, in all four shapes (unary and the three streaming ones); a failure is a `CallError` with a `StatusCode`, metadata is `Metadata` (many values per key, binary entries)
- **`GrpcChannelConfig`**: typed channel configuration: endpoint, TLS (required by default), keep-alive, message size limit, compression, request timeout
- **`TonicGrpcEgressBuilderSpiFactory`**: the one way to build a client: a factory for the endpoint hands out a builder with a `with_*` option for every capability (TLS and mTLS, keep-alive, message limits, send and accept compression, request timeout, interceptors, trace context, load balancer), and `build` returns the client
- **`GrpcEgressInterceptor`** and its chain: hooks before and after each call and observes Pattern lifecycle events through `on_event`; the trace-context interceptor (created by `StdGrpcEgressInterceptorFactory`) injects W3C trace headers when `with_trace_context` attaches it. The adapter preserves fallible `after_call` behavior ([ADR-012](adr/ADR-012-pattern-egress-lifecycle-adapter.md))
- Workspace crates: `transport`, `retry`, `breaker`, `resilient`, `auth-bearer`, each a domain crate and an implementation crate (`<name>-std`)

## WHY

| Problem | Solution |
|---------|----------|
| Channel setup (TLS, keep-alive, deadlines, message cap) repeated in every service | `GrpcChannelConfig` and the builder |
| Retry and circuit breaking implemented per client | the `retry` and `breaker` crates wrap any `Transport + RawTransport`; the `resilient` crate applies both |
| Trace headers dropped on outbound calls | the builder's `with_trace_context` attaches the interceptor that injects them |
| One client per backend and no balancing | the load-balanced client selects among several backends and tracks their health |
| Bearer tokens minted by hand | the `auth-bearer` crate signs a JWT from configured claims and adds it to each request |

## Verification records

The current mutation manifest has 128 entries: 122 caught, 6 documented expected survivors, and 0 invalid or unexplained survivors. The issue #16 composition-root scenario rerun and per-mutant results are in [mutation-results.md](mutation-results.md). The Pattern lifecycle adapter decision and implementation are in [ADR-012](adr/ADR-012-pattern-egress-lifecycle-adapter.md).

## HOW

The transport builder creates typed and raw clients through the `proto-svc` engine. Configure the endpoint, security, limits, compression, timeout, balancing, and interceptors before calling `build()`. Apply retry or breaker after construction with their separate factories. The [root README](../../README.md) has consumer examples and workspace commands; the [composition-root example](../main/examples/composition-root/README.md) demonstrates all nine scenarios.
