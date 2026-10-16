# edge-transport-grpc-egress

> **TLDR:** Outbound gRPC client for swe-edge: one `GrpcEgress` trait, a `tonic`-backed client with a client-side load balancer, and retry, circuit-breaker and bearer-auth pieces as separate crates. See [Overview](scm/docs/README.md) for details.

## Crates

The workspace is `scm/`; each crate is in `scm/main/<crate>/`.

| Crate | What it is |
|-------|------------|
| `grpc-egress-transport` | the domain: the `GrpcEgress` trait, request and response types, errors and the options of a client. Plain data and traits, no `tonic`, `hyper` or `rustls` |
| `grpc-egress-transport-std` | the implementation: the `hyper` client and its builder, the load-balanced client, interceptors, and the constructors of the domain types |
| `grpc-egress-retry` | the retry domain: contracts, requests, responses, errors and the retry policy (`GrpcRetryConfig`) |
| `grpc-egress-retry-std` | retry decorator around any `GrpcEgress` |
| `grpc-egress-breaker` | the circuit-breaker domain: contracts, requests, responses, errors and the breaker policy (`GrpcBreakerConfig`) |
| `grpc-egress-breaker-std` | circuit-breaker decorator around any `GrpcEgress` |
| `grpc-egress-resilient-std` | wraps a client in retry and breaker together (`GrpcResilientFacade::apply_resilience`) |
| `grpc-egress-auth-bearer-std` | JWT bearer interceptor: signs a token and adds `authorization: Bearer <jwt>` to each outbound request |

## Usage

```rust,ignore
use std::time::Duration;
use grpc_egress_transport::{GrpcChannelConfig, GrpcEgress, GrpcRequest};
use grpc_egress_transport_std::{GrpcChannelConfigFactory, GrpcRequestFactory, TransportConstruction};

let config = GrpcChannelConfig::new("https://api.example.com:443");
let transport = TransportConstruction::create_transport_from_config(&config)?;
// transport: Arc<dyn GrpcEgress>

let request = GrpcRequest::new("/pkg.Service/Method", body_bytes, Duration::from_secs(10));
let response = transport.call_unary(request).await?;
```

A runnable version of the first part is `scm/main/transport-std/examples/basic_transport.rs`.

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
