# edge-transport-grpc-egress

> **TLDR:** Outbound gRPC client for swe-edge: a client that is a `proto-pattern` `Transport` and `RawTransport` over the `proto-svc` engine, with a client-side load balancer built through one builder, and retry, circuit-breaker and bearer-auth pieces as separate crates. See [Overview](scm/docs/README.md) for details.

**Audience:** Rust developers integrating the egress client, and maintainers working on its domain and implementation crates.

## Table of Contents

- [WHAT](#what)
- [WHY](#why)
- [HOW](#how)
- [Crates](#crates)
- [Options](#options-of-the-client-builder)
- [Building and checking](#building-and-checking)

## WHAT

A Rust workspace for outbound gRPC clients that implement Pattern typed and raw transport contracts over the `proto-svc` engine. It also provides load balancing and separately composable retry, breaker, and bearer-auth crates.

## Crates

The workspace is `scm/`; each crate is in `scm/main/<crate>/`. Every contract is its own `-api` package
(traits, plain data, errors; no implementation), separate from its `-std` and/or `-spi` implementation packages,
which depend on it
([ADR-006](scm/docs/adr/ADR-006-domain-first.md)).

| Crate | What it is |
|-------|------------|
| `grpc-egress-transport-api` | the domain: the client builder trait, the options of a client, the interceptor trait, the factories of the cancellation and security-context views, and the channel configuration. The calls, requests, responses, errors, status codes and metadata are the contract's (`proto-pattern`). No `tonic`, `hyper` or `rustls` |
| `grpc-egress-transport-std` | the implementation: the builder over the `proto-svc` engine, the load-balanced client, interceptors, the cancellation and security-context views, the `grpc.health.v1` health-check call, the reading of TLS files, and the constructors of the domain types |
| `grpc-egress-retry-api` / `-std` | retry: `GrpcRetryFactory`, `StdGrpcRetryFactory`, the retry policy `GrpcRetryConfig`; wraps any `Transport + RawTransport` |
| `grpc-egress-breaker-api` / `-std` | circuit breaker: `GrpcBreakerFactory`, `StdGrpcBreakerFactory`, the policy `GrpcBreakerConfig`, `BreakerObservable`; wraps any `Transport + RawTransport` |
| `grpc-egress-resilient-api` / `-std` | retry and breaker together: `StdGrpcResilientFactory`, the policy `ResilienceConfig` |
| `grpc-egress-auth-bearer-api` / `-std` | JWT bearer: `StdBearerInterceptorFactory` signs a token and adds `authorization: Bearer <jwt>` to each request |
| `grpc-egress-test-support` | fixtures for tests: an in-process peer (echo, reply, delay, status, TLS, mTLS), a freezable proxy |
| `grpc-egress-example-composition-root` | what a composition root does with these crates: nine tested scenarios (`scm/main/examples/composition-root`) |

## WHY

The workspace gives service authors one typed and raw client construction path while keeping transport mechanics, retry, breaker, and bearer authentication in their owning crates. This makes options composable and lets policies wrap the same client contract.

## HOW

### Quick Start


One way to build a client: a factory for the endpoint hands out a builder; every capability is a `with_*`
option; `build()` validates and returns the client, naming the options of any refusal.

```rust,ignore
use std::time::Duration;
use grpc_egress_transport_api::{
    BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;
use proto_pattern::{CallOptions, CallRequest, CallRawRequestBuilder, Compression, MethodPath, RawTransport, Transport};

let client = TonicGrpcEgressBuilderSpiFactory::new("https://api.example.com:443")
    .build(BuildGrpcEgressBuilderRequest)
    .with_send_compression(Compression::Gzip)
    .with_accept_compression(Compression::Gzip)
    .with_request_timeout(Duration::from_secs(10))
    .build()?;

// By path, with bytes for messages (a path known only at run time):
let path = MethodPath::create("/pkg.Service/Method")?;
let response = client
    .call_raw(CallRawRequestBuilder::new()
        .with_path(path)
        .with_message(body_bytes)
        .with_options(CallOptions::default())
        .build())
    .await?;

// Or typed, over a `Method` the caller defines:
let response = client
    .call::<Greet>(CallRequest::new(request, CallOptions::default()))
    .await?;
```

The client is a `Transport` and a `RawTransport` at once, over one connection pool: a caller holds it through a
generic bound, never a `dyn`. A failure is a `CallError`, with a `StatusCode` on every variant.

Retry and breaker wrap a built client; the transport knows neither
([ADR-004](scm/docs/adr/ADR-004-remove-resilience-config-from-transport-primitive.md),
[ADR-007](scm/docs/adr/ADR-007-decorators-wrap-a-built-client.md)):

```rust,ignore
let guarded = StdGrpcResilientFactory::new(resilience_config)
    .build(BuildGrpcResilientRequest { inner: client })?;
```

Runnable versions: `scm/main/transport-std/examples/basic_transport.rs`,
`scm/main/resilient-spi/examples/resilient.rs` and the composition-root example
(`cargo run -p grpc-egress-example-composition-root`).

### Scope


This crate ships the client side of the transport: the client over the `proto-svc` engine (which owns the
wire format, the connection, TLS, compression and the status codes), the load balancer and interceptors. It does **not** ship
server machinery: it binds no listener, runs no accept loop, terminates no TLS and owns no connection
lifecycle on the serving side. That is the job of the deploying application. The in-process peer in `grpc-egress-test-support` is for tests and is not part of any
published crate.

### Options of the client builder

`with_endpoint`, `with_plaintext_allowed`, `with_request_timeout`, `with_max_decoding_message_size`,
`with_max_encoding_message_size`, `with_send_compression`, `with_accept_compression`, `with_tls`, `with_keep_alive`,
`with_loadbalancer`, `with_interceptor`, `with_trace_context`, `with_channel_config` (a loaded
`GrpcChannelConfig`, then single options override it). Two views wrap a built client for one purpose each: a
cancellation signal (`GrpcCancellationFactory`) and a caller identity (`GrpcCallerIdentityFactory`). The
`grpc.health.v1` health-check call is `HealthCheck`.

### Layout

```
scm/
  Cargo.toml                    the workspace (members listed one by one)
  CHANGELOG.md
  main/<theme>-api/              contract package: src/<theme folder>/{dto,vo,error,traits}/, tests/
  main/<theme>-std/             implementation crate: src/{std,spi,saf}/, tests/, examples/
  main/examples/composition-root/
  docs/                         architecture and ADRs
scripts/                        check_layout.py, rename_crate.py, mutation/ (run_mutants.py and mutants.json)
.githooks/                      pre-commit: fmt, clippy, arch audit, cargo audit, layout, script tests
```

### Documentation

- [Workspace overview](scm/docs/README.md)
- [Architecture](scm/docs/architecture.md)
- [Behavior comparison](scm/docs/behaviour-comparison.md)
- [Conformance record](scm/docs/egress-conformance.md)
- [Mutation results](scm/docs/mutation-results.md)
- [Changelog](scm/CHANGELOG.md)

### Building and checking

```bash
cd scm
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo run -p grpc-egress-example-composition-root
cd .. && python scripts/check_layout.py
```

## Summary

Build a client with `GrpcEgressBuilder`, then apply retry or breaker policies around the resulting `Transport + RawTransport`. See the [composition-root example](scm/main/examples/composition-root/README.md) for a complete public-API consumer.
