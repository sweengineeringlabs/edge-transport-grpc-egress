# edge-transport-grpc-egress

> **TLDR:** Outbound gRPC client for swe-edge: one `GrpcEgress` trait, a `tonic`-backed client with a client-side load balancer, and retry, circuit-breaker and bearer-auth pieces as separate crates. See [Overview](scm/docs/README.md) for details.

## Crates

The workspace is `scm/`; each crate is in `scm/main/<crate>/`.

| Crate | What it is |
|-------|------------|
| `edge-transport-grpc-egress-transport` | the `GrpcEgress` trait, request and response types, the `tonic` client, a load-balanced client, the interceptor chain and a trace-context interceptor |
| `edge-transport-grpc-egress-retry` | retry decorator around any `GrpcEgress` |
| `edge-transport-grpc-egress-breaker` | circuit-breaker decorator around any `GrpcEgress` |
| `edge-transport-grpc-egress-resilient` | wraps a client in retry and breaker together (`GrpcResilientFacade::apply_resilience`) |
| `edge-transport-grpc-egress-auth-bearer` | JWT bearer interceptor: signs a token and adds `authorization: Bearer <jwt>` to each outbound request |

## Usage

```rust,ignore
use std::time::Duration;
use edge_transport_grpc_egress_transport::{
    GrpcChannelConfig, GrpcEgress, GrpcRequest, TransportConstruction,
};

let config = GrpcChannelConfig::new("https://api.example.com:443");
let transport = TransportConstruction::create_transport_from_config(&config)?;
// transport: Arc<dyn GrpcEgress>

let request = GrpcRequest::new("/pkg.Service/Method", body_bytes, Duration::from_secs(10));
let response = transport.call_unary(request).await?;
```

A runnable version of the first part is `scm/main/transport/examples/basic_transport.rs`.

## Public surface of the transport crate

| Export | Purpose |
|--------|---------|
| `GrpcEgress` | the outbound trait: `call_unary`, `call_server_stream`, `call_client_stream`, `call_bidi_stream`, `health_check` |
| `GrpcRequest` / `GrpcResponse` | raw protobuf bytes and metadata; every request carries a mandatory deadline |
| `GrpcChannelConfig` | endpoint, TLS (required by default), keep-alive, message size cap, compression, request timeout |
| `TransportConstruction` | factory functions: the plain client, the load-balanced client, and (feature `prost`) the prost client |
| `GrpcEgressError` / `GrpcStatusCode` | the error type and the 17 gRPC status codes |
| `GrpcEgressInterceptor` / `GrpcEgressInterceptorChain` | hooks run before and after each call |
| `TraceContextGrpcEgressInterceptor` | injects the W3C `traceparent` and `tracestate` headers |

## Known limits

These are tracked, not hidden:

- `GrpcChannelConfig::mtls` is accepted but not applied: the client does not read it (issue 13).
- No interceptor is attached to a client by default, trace context included, and the public API has no way to attach one. The builder of issue 16 adds it.
- The crates are being moved to a new layout and construction API (issues 16, 17 and 18).

## Layout

```
scm/
  Cargo.toml                 the workspace
  main/<crate>/
    Cargo.toml
    src/{api,core,spi,saf}/   layers of the crate
    tests/  examples/  config/
  docs/                      architecture and ADRs
  scripts/                   CI scripts and git hooks
```

## Building

```bash
cd scm
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
```
