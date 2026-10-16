# edge-transport-grpc-egress

## WHAT

Outbound gRPC client for swe-edge: a `tonic`-backed client behind one trait, with retry and
circuit-breaker decorators and a bearer-auth interceptor as separate crates.

Key capabilities:

- **`GrpcEgress`**: the outbound trait (unary and the three streaming shapes, plus a health check)
- **`GrpcRequest`** / **`GrpcResponse`**: raw protobuf bytes and metadata; each request has a mandatory deadline and an optional cancellation token
- **`GrpcChannelConfig`**: typed channel configuration: endpoint, TLS (required by default), keep-alive, message size cap, compression mode, request timeout
- **`TonicGrpcEgressBuilderSpiFactory`**: the one way to build a client: a factory for the endpoint hands out a builder with a `with_*` option for every capability (TLS and mTLS, keep-alive, message cap, compression, request timeout, interceptors, trace context, load balancer, prost codec), and `build` returns the client
- **`GrpcEgressInterceptor`** and its chain: hooks before and after each call; **`TraceContextGrpcEgressInterceptor`** injects the W3C trace headers when it is attached
- Workspace crates: `transport`, `retry`, `breaker`, `resilient`, `auth-bearer`, each a domain crate and an implementation crate (`<name>-std`)

## WHY

| Problem | Solution |
|---------|----------|
| Channel setup (TLS, keep-alive, deadlines, message cap) repeated in every service | `GrpcChannelConfig` and the builder |
| Retry and circuit breaking implemented per client | the `retry` and `breaker` crates wrap any `GrpcEgress`; the `resilient` crate applies both |
| Trace headers dropped on outbound calls | `TraceContextGrpcEgressInterceptor` injects them; it is not attached by default (see Known limits in the root README) |
| One client per backend and no balancing | the load-balanced client selects among several backends and tracks their health |
| Bearer tokens minted by hand | the `auth-bearer` crate signs a JWT from configured claims and adds it to each request |
