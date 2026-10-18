# edge-transport-grpc-egress

## WHAT

Outbound gRPC client for swe-edge: a client over the `proto-svc` engine that is a `proto-pattern` `Transport`
and `RawTransport`, with retry and circuit-breaker decorators and a bearer-auth interceptor as separate crates.

Key capabilities:

- **`Transport`** and **`RawTransport`** (`proto-pattern`): the client the builder returns is both: a call over a `Method`, and a call to a `MethodPath` with bytes for messages, in all four shapes (unary and the three streaming ones); a failure is a `CallError` with a `StatusCode`, metadata is `Metadata` (many values per key, binary entries)
- **`GrpcChannelConfig`**: typed channel configuration: endpoint, TLS (required by default), keep-alive, message size limit, compression, request timeout
- **`TonicGrpcEgressBuilderSpiFactory`**: the one way to build a client: a factory for the endpoint hands out a builder with a `with_*` option for every capability (TLS and mTLS, keep-alive, message limits, send and accept compression, request timeout, interceptors, trace context, load balancer), and `build` returns the client
- **`GrpcEgressInterceptor`** and its chain: hooks before and after each call; the trace-context interceptor (created by `StdGrpcEgressInterceptorFactory`) injects the W3C trace headers when `with_trace_context` attaches it
- Workspace crates: `transport`, `retry`, `breaker`, `resilient`, `auth-bearer`, each a domain crate and an implementation crate (`<name>-std`)

## WHY

| Problem | Solution |
|---------|----------|
| Channel setup (TLS, keep-alive, deadlines, message cap) repeated in every service | `GrpcChannelConfig` and the builder |
| Retry and circuit breaking implemented per client | the `retry` and `breaker` crates wrap any `Transport + RawTransport`; the `resilient` crate applies both |
| Trace headers dropped on outbound calls | the builder's `with_trace_context` attaches the interceptor that injects them |
| One client per backend and no balancing | the load-balanced client selects among several backends and tracks their health |
| Bearer tokens minted by hand | the `auth-bearer` crate signs a JWT from configured claims and adds it to each request |
