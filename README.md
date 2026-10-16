# edge-transport-grpc-egress

> **TLDR:** Outbound gRPC client for swe-edge: one `GrpcEgress` trait, a `hyper`-backed client with a client-side load balancer built through one builder, and retry, circuit-breaker and bearer-auth pieces as separate crates. See [Overview](scm/docs/README.md) for details.

## Crates

The workspace is `scm/`; each crate is in `scm/main/<crate>/`. Every theme is a pair: a domain crate
(traits, plain data, errors; no implementation) and an implementation crate (`-std`) that depends on it
([ADR-006](scm/docs/adr/ADR-006-domain-first.md)).

| Crate | What it is |
|-------|------------|
| `grpc-egress-transport` | the domain: the `GrpcEgress` trait, the client builder trait, requests and responses, errors, and the options of a client. No `tonic`, `hyper` or `rustls` |
| `grpc-egress-transport-std` | the implementation: the `hyper` client, its builder, the load-balanced client, interceptors, and the constructors of the domain types |
| `grpc-egress-retry` / `-std` | retry: `GrpcRetryFactory`, `StdGrpcRetryFactory`, the retry policy `GrpcRetryConfig` |
| `grpc-egress-breaker` / `-std` | circuit breaker: `GrpcBreakerFactory`, `StdGrpcBreakerFactory`, the policy `GrpcBreakerConfig`, `BreakerObservable` |
| `grpc-egress-resilient` / `-std` | retry and breaker together: `StdGrpcResilientFactory`, the policy `ResilienceConfig` |
| `grpc-egress-auth-bearer` / `-std` | JWT bearer: `StdBearerInterceptorFactory` signs a token and adds `authorization: Bearer <jwt>` to each request |
| `grpc-egress-test-support` | fixtures for tests: an in-process peer (echo, reply, delay, status, TLS, mTLS), a freezable proxy |
| `grpc-egress-example-composition-root` | what a composition root does with these crates: seven tested scenarios (`scm/main/examples/composition-root`) |

## Usage

One way to build a client: a factory for the endpoint hands out a builder; every capability is a `with_*`
option; `build()` validates and returns the client, naming the options of any refusal.

```rust,ignore
use std::time::Duration;
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, CompressionMode, GrpcEgress, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, GrpcRequest,
};
use grpc_egress_transport_std::{GrpcRequestFactory, TonicGrpcEgressBuilderSpiFactory};

let client = TonicGrpcEgressBuilderSpiFactory::new("https://api.example.com:443")
    .build(BuildGrpcEgressBuilderRequest)
    .with_compression(CompressionMode::Gzip)
    .with_request_timeout(Duration::from_secs(10))
    .build()?;

let request = GrpcRequest::new("/pkg.Service/Method", body_bytes, Duration::from_secs(10));
let response = client.call_unary(request).await?;
```

Retry and breaker wrap a built client; the transport knows neither
([ADR-004](scm/docs/adr/ADR-004-remove-resilience-config-from-transport-primitive.md),
[ADR-007](scm/docs/adr/ADR-007-decorators-wrap-a-built-client.md)):

```rust,ignore
let guarded = StdGrpcResilientFactory::new(resilience_config)
    .build(BuildGrpcResilientRequest { inner: client })?;
```

Runnable versions: `scm/main/transport-std/examples/basic_transport.rs`,
`scm/main/resilient-std/examples/resilient.rs` and the composition-root example
(`cargo run -p grpc-egress-example-composition-root`).

## Options of the client builder

`with_endpoint`, `with_plaintext_allowed`, `with_request_timeout`, `with_max_message_bytes`,
`with_compression`, `with_mtls`, `with_root_ca`, `with_keep_alive`, `with_loadbalancer`,
`with_prost_codec`, `with_interceptor`, `with_trace_context`, `with_channel_config` (a loaded
`GrpcChannelConfig`, then single options override it). `with_prost_codec` needs the Cargo feature `prost`;
without it `build()` returns `DisabledFeatureOptions` naming the option and the feature.

## Layout

```
scm/
  Cargo.toml                    the workspace (members listed one by one)
  CHANGELOG.md
  main/<theme>/                 domain crate: src/<theme folder>/{dto,vo,error,traits}/, tests/
  main/<theme>-std/             implementation crate: src/{std,spi,saf}/, tests/, examples/, config/
  main/examples/composition-root/
  docs/                         architecture and ADRs
scripts/                        check_layout.py, rename_crate.py, mutation_proof_*.py
.githooks/                      pre-commit: fmt, clippy, arch audit, cargo audit, layout, script tests
```

## Building and checking

```bash
cd scm
cargo build
cargo test --workspace
cargo test -p grpc-egress-transport-std --features prost
cargo clippy --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo run -p grpc-egress-example-composition-root
cd .. && python scripts/check_layout.py
```
