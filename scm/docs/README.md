# edge-transport-grpc-egress

## WHAT

Outbound gRPC client for swe-edge: a `tonic`-backed client behind one trait, with retry and
circuit-breaker decorators and a bearer-auth interceptor as separate crates.

Key capabilities:

- **`GrpcEgress`**: the outbound trait (unary and the three streaming shapes, plus a health check)
- **`GrpcRequest`** / **`GrpcResponse`**: raw protobuf bytes and metadata; each request has a mandatory deadline and an optional cancellation token
- **`GrpcChannelConfig`**: typed channel configuration: endpoint, TLS (required by default), keep-alive, message size cap, compression mode, request timeout
- **`TransportConstruction`**: factory functions for the plain client and the load-balanced client (client-side selection over several backends)
- **`GrpcEgressInterceptor`** and its chain: hooks before and after each call; **`TraceContextGrpcEgressInterceptor`** injects the W3C trace headers when it is attached
- **`ResilientGrpcClientPort`**: read access to a resilient client's state (`circuit_state`, `consecutive_failures`, `last_error`)
- Workspace crates: `transport`, `retry`, `breaker`, `resilient`, `auth-bearer`

## WHY

| Problem | Solution |
|---------|----------|
| Channel setup (TLS, keep-alive, deadlines, message cap) repeated in every service | `GrpcChannelConfig` and the factory functions of `TransportConstruction` |
| Retry and circuit breaking implemented per client | the `retry` and `breaker` crates wrap any `GrpcEgress`; the `resilient` crate applies both |
| Trace headers dropped on outbound calls | `TraceContextGrpcEgressInterceptor` injects them; it is not attached by default (see Known limits in the root README) |
| One client per backend and no balancing | the load-balanced client selects among several backends and tracks their health |
| Bearer tokens minted by hand | the `auth-bearer` crate signs a JWT from configured claims and adds it to each request |
