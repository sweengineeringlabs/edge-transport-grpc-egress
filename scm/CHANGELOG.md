# Changelog

All notable changes to the workspace are documented here. The per-crate `CHANGELOG.md` files of the
`-std` crates hold the history of their own crate before 0.8.0.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and the workspace
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). A tag is cut only on the owner's
word.

## [Unreleased]

### Issue 65: use load-balancer v0.5.0

- Update the transport SPI's load-balancer dependency to v0.5.0, which removes the configbuilder dependency from the load-balancer primitive.

### Issue 64: give contract packages explicit API boundaries

- Rename each contract crate to its `grpc-egress-<theme>-api` package and `_api` Rust crate name, separate from its `-std` and `-spi` implementation packages.
- Move contract modules directly under `src/`, update consumers and workspace metadata, and enforce the package and source layout with the migration and layout-check scripts.
- Consumers import root-re-exported contract types from `grpc_egress_<theme>_api`; there is no `api::` source module path. The type names and root-level exports are retained.

## [0.12.0] - 2026-10-23

### Issue 63: separate standard and SPI implementation packages

- Split `auth-bearer`, `retry`, and `transport` implementation layers into distinct `-std` and `-spi` packages.
- Rename SPI-only `breaker-std` and `resilient-std` packages to `breaker-spi` and `resilient-spi`; no empty standard packages are created.
- Update workspace consumers to choose the packages they use. Implementations that previously crossed crate-private boundaries now compose through public domain factory contracts.
- The implementation package root exports only its SAF factories; SPI provider APIs move to the new `-spi` crate paths.

## [0.11.0] - 2026-10-23

### Issue 60 follow-up: implementation types belong behind domain interfaces

- **Breaking API boundary:** `HealthCheck`, `HealthCheckRequest`, `HealthCheckResponse`, and `HealthStatus` are now exported from `grpc_egress_transport_api`; their old `grpc_egress_transport_std` paths are removed. They retain the standard `grpc.health.v1` wire format.
- **Breaking API boundary:** `grpc_egress_transport_std` no longer re-exports `TokenCancellation`, `SecurityContextIdentity`, or `edge_security_runtime::SecurityContext`. Implement `CancellationSignal` and `CallerIdentity` for a caller-owned type, and import `SecurityContext` from `edge_security_runtime` directly. The composition-root example shows both adapters.
- The implementation crate root now exports its SAF factories only. `JustRag #53` consumes the builder factory and traits, which remain unchanged; no external build was available in this workspace.

### Issue 61: Pattern call lifecycle adapter

- Route typed and raw call identities/options through Pattern v0.12.0 `TransportInterceptor` and report stream open, message, completion, failure, and cancellation events to registered egress interceptors.
- Keep the existing `GrpcEgressInterceptor` registration and fallible `after_call` behavior. The added event callback is observational and uses Pattern `CallEvent` with tonic's concrete `Metadata`, which keeps runtime registration object-safe and preserves existing implementors through a default method.
- Migration: existing `with_interceptor` and `with_trace_context` call sites remain unchanged. New event observers may implement `GrpcEgressInterceptor::on_event`; there is no path rewrite API in Pattern's pre-call contract.
- The egress builder remains the consumer facade; portable construction settings still flow into Pattern's `TransportConfiguration` through the tonic engine, while keep-alive remains an engine option.

### Issue 62: validated, immutable configuration values

- **Breaking for direct struct construction and field access.** Configuration value objects now keep state private and expose read-only accessors. Use their validated constructors, existing builders, or the documented serde representation.
- Retry and breaker configs validate at construction and deserialization. `ResilienceConfig::from_policies` composes already validated retry and breaker policies.
- `GrpcRetryConfig::try_new` now accepts a `BuildGrpcRetryConfigRequest` construction bundle; the standard fluent retry-config builder remains available.
- `BearerSecret::hs256` and `BearerSecret::rs256` replace public enum variants and reject empty key material. The tagged serialized shape remains compatible.
- Channel, keep-alive, mTLS, backend-target and load-balancer settings now validate all public construction and deserialization paths. `GrpcChannelConfig::default()` remains an incomplete builder seed; provide an endpoint before building.
- `GrpcBreakerConfig` adds `enabled` (default `true`). When false, calls pass through without breaker state/admission changes.
- Request/response DTO fields are private where required by the pinned architecture; use their constructors and accessors.

### Proto dependency upgrade

- Pin every consumer to `proto-pattern` v0.12.0 and the `proto-svc` crates v0.4.2.
- Migrate raw and streaming calls, stream reads, and metadata queries to their operation-specific request/response types; typed unary requests and responses use `new`.
- Build the engine through `TonicTransportSpiFactoryOptions::tonic_dual_builder`, `TransportConfiguration`, and `DualTransportBuilder`, retaining the existing keep-alive options.
- Propagate metadata query and binary decoding errors when preparing interceptor replies.

## [0.10.0] - 2026-10-22

### Security

Updated `webpki-roots` to 1.0.10 and `rustls-webpki` to 0.103.16 in the lockfile to resolve RustSec-2026-0381 and RustSec-2026-0382.

### Issue 60: programming over the interface (ADR-010)

Everything another module calls is defined in a domain crate;
`std/` and `spi/` only implement those traits on `pub(crate)` types; `std/` names no third-party crate. No method is
removed and no behaviour changes: every operation below is reachable by the new way, and the tests that exercised
the old path keep their names. `TonicGrpcEgressBuilderSpiFactory::new(endpoint)`, the builder traits and every `saf/`
factory constructor (`StdGrpcRetryFactory::new`, `StdGrpcBreakerFactory::new`, ...) are unchanged.

### Breaking (layout)

Each row is a public path of a `-std` crate that no longer exists, and the way to do the same. A "factory" is a `saf/`
type of the same crate that implements the domain factory trait of the same name; build its product with
`factory.build(BuildXRequest)`. Import the domain trait to call its methods.

| Old path | New way |
|---|---|
| `grpc_egress_transport_std::GrpcChannelConfigFactory` (`GrpcChannelConfig::new`, `allow_plaintext`, `with_mtls`, `with_keep_alive`, `with_max_message_bytes`, `with_compression`, `with_request_timeout`) | domain trait `grpc_egress_transport_api::GrpcChannelConfigBuilder`, created by `StdGrpcChannelConfigBuilderFactory`: `.with_endpoint(e).allow_plaintext()....build()?` (`build` is fallible: the endpoint is required). `GrpcChannelConfig` stays plain data with public fields |
| `grpc_egress_transport_std::GrpcChannelConfigBuilder` (type, `new`) | the same domain trait, `GrpcChannelConfigBuilder`; `new()` is `StdGrpcChannelConfigBuilderFactory.build(BuildGrpcChannelConfigBuilderRequest)`; the builder also has `with_request_timeout` now |
| `grpc_egress_transport_std::MtlsConfigFactory` (`MtlsConfig::new`, `MtlsConfig::with_root_ca`) | `grpc_egress_transport_api::MtlsConfigFactory`, implemented by `StdMtlsConfigFactory`: `build(BuildMtlsConfigRequest { cert_pem_path, key_pem_path, ca_pem_path })`, `ca_pem_path: None` for `new` |
| `grpc_egress_transport_std::TlsSettingsResolver::resolve(identity, root_ca)` | `grpc_egress_transport_api::TlsSettingsResolver::resolve(ResolveTlsSettingsRequest { identity, root_ca })`, created by `StdTlsSettingsResolverFactory` |
| `grpc_egress_transport_std::LoadbalancerSettingsProvider` (`LoadbalancerSettings::from_config`) | `grpc_egress_transport_api::LoadbalancerSettingsProvider::parse_config(text)`, created by `StdLoadbalancerSettingsProviderFactory` (`from_config` is renamed because clippy forbids a `from_*` method that takes `&self`) |
| `grpc_egress_transport_std::GrpcEgressInterceptorChain` (`new`, `push`, `len`, `is_empty`, `run_before`, `run_after`) | domain trait `grpc_egress_transport_api::GrpcEgressInterceptorChain`, created by `StdGrpcEgressInterceptorChainFactory` |
| `grpc_egress_transport_std::TraceContextGrpcEgressInterceptor` (`pass_through`, `with_static`) | `StdGrpcEgressInterceptorFactory::new(TraceContextSource::PassThrough or Static { .. }).build(BuildGrpcEgressInterceptorRequest)`; the type is `pub(crate)` |
| `grpc_egress_transport_std::{HealthCheck, HealthCheckRequest, HealthCheckResponse, HealthStatus}` | `grpc_egress_transport_api::{HealthCheck, HealthCheckRequest, HealthCheckResponse, HealthStatus}`; health payloads retain the `grpc.health.v1` wire format |
| `grpc_egress_transport_std::TokenCancellation` | implement the domain `CancellationSignal` trait on a caller-owned wrapper around the caller's cancellation token |
| `grpc_egress_transport_std::{SecurityContextIdentity, SecurityContext}` | import `SecurityContext` from `edge_security_runtime` and implement `grpc_egress_transport_api::CallerIdentity` on a caller-owned wrapper |
| `grpc_egress_retry_std::GrpcRetryConfigProvider` (`GrpcRetryConfig::from_config`, `initial_backoff`, `max_backoff`, `rate_limit_initial_backoff`, `rate_limit_max_backoff`, `validate`) | `grpc_egress_retry_api::GrpcRetryConfigProvider` (`parse_config(text)`, and the others take the config as an argument), created by `StdGrpcRetryConfigProviderFactory` |
| `grpc_egress_retry_std::GrpcRetryConfigBuilder` (type, `new`) | domain trait `grpc_egress_retry_api::GrpcRetryConfigBuilder`, created by `StdGrpcRetryConfigBuilderFactory` |
| `grpc_egress_retry_std::RetryDecision` (`classify`, `should_retry`, `is_rate_limit`, `read_retry_after`, `parse_retry_after_hint`) and `ResourceExhaustedContext::classify` | the enums `RetryDecision` and `ResourceExhaustedContext` are values of `grpc_egress_retry_api`; the operations are methods of `grpc_egress_retry_api::RetryClassifier` (`classify`, `should_retry(decision)`, `is_rate_limit(decision)`, `read_retry_after`, `parse_retry_after_hint`, `classify_resource_exhausted`), created by `StdRetryClassifierFactory` |
| `grpc_egress_breaker_std::GrpcBreakerConfigProvider` (`GrpcBreakerConfig::from_config`, `cool_down`, `validate`) | `grpc_egress_breaker_api::GrpcBreakerConfigProvider` (`parse_config`, `cool_down(&config)`, `validate(&config)`), created by `StdGrpcBreakerConfigProviderFactory` |
| `grpc_egress_resilient_std::ResilienceConfigProvider` (`ResilienceConfig::from_config`, `validate`, `to_retry_config`, `to_breaker_config`) | `grpc_egress_resilient_api::ResilienceConfigProvider` (`parse_config`, and the others take the config), created by `StdResilienceConfigProviderFactory`. `grpc-egress-resilient-api` now depends on `grpc-egress-retry-api` (a domain crate) |
| `grpc_egress_auth_bearer_std::BearerSecretGuard` (`secret.ct_eq_hs256(&other)`) | `grpc_egress_auth_bearer_api::BearerSecretGuard::ct_eq_hs256(&left, &right)`, created by `StdBearerSecretGuardFactory` |
| `grpc_egress_auth_bearer_std::BearerEgressConfigBuilder` and `JwtClaimsBuilder` (types, `new`) | domain traits of the same names in `grpc_egress_auth_bearer_api`, created by `StdBearerEgressConfigBuilderFactory` and `StdJwtClaimsBuilderFactory` |

### Downstream (issue 60, task 9)

- **In this workspace**, every consumer builds and passes: the five domain and five `-std` crates, `grpc-egress-test-support`,
  the example `grpc-egress-example-composition-root` and the five `examples/` of the `-std` crates
  (`cargo build --workspace --examples` and each run, exit 0). The example and the tests use the public path only.
- **External**, none migrated here, none affected before it bumps the tag: `edge-bootstrap` (pins
  `edge-transport-grpc-egress-transport` 0.6.2 plus retry and breaker), `edge-bench` (pins 0.5.1 of the old
  `edge-egress-grpc`), and JustRag (#53), which builds through `TonicGrpcEgressBuilderSpiFactory::new(endpoint)` and the
  builder traits `GrpcEgressBuilderFactory` and `GrpcEgressBuilder`: both are unchanged by issue 60 (same crate, same
  path, same methods), so JustRag needs no change for this issue. Anything it calls from the table above (a channel
  configuration, `MtlsConfig`, a retry or breaker policy parsed from TOML) changes as the table says.

New in the domain crates: `pub use proto_pattern;` (the contract the interfaces are written over, so that an
implementation depends on the domain crate only); `grpc_egress_transport_api::MetadataExtender`;
`grpc_egress_breaker_api::FailureClassifier` and `Outcome`. In `-std` crates the code that names a third-party crate
moved from `std/` to `spi/` (no public path changed for it, except as listed above). `scripts/check_layout.py` has
rules 19 (`program_over_the_interface`), 20 (`std_names_no_third_party`) and 21 (`every_function_a_method`: no free function in `src/`). `deny.toml` lists the two git repositories (`proto-pattern`, `proto-svc`) that made `cargo deny check` fail on `sources`.


Issue 18: the contract (`proto-pattern` 0.10) and the engine (`proto-svc` 0.4) are external, and the client is a
generic bound over them. **Breaking throughout.** The design is ADR-009 (rulings of ADR-008 applied); every
behaviour that differs from v0.8.0 and its decision is in `docs/behaviour-comparison.md`; the mapping of every public
item is `docs/contract-mapping.md`.

### Changed (breaking)

- **The client is a `Transport` and a `RawTransport`**, built by `GrpcEgressBuilder::build` over `proto-svc`'s
  `DualTransportFactory` (one engine, one connection pool). `GrpcEgress`, its `BoxFuture` methods and every
  `Arc<dyn GrpcEgress>` are gone: a caller holds a generic bound. The hand-written `hyper` client, its frame codec and its
  TLS connector are removed. `rust-version` is 1.99.
- **Call types are the contract's.** `GrpcRequest`, `GrpcResponse`, `CallStreamRequest`, `GrpcMessageStreamResponse`
  become `MethodPath` + `CallRequest<Vec<u8>>` + `CallOptions`, `CallResponse`, `impl ResponseStream` and `impl
  MessageStream`. `GrpcEgressError`/`GrpcEgressResult` become `CallError` (a status code on every failure),
  `GrpcStatusCode` becomes `StatusCode`, and the `HashMap<String, String>` metadata becomes `Metadata`: many values per
  key and binary (`-bin`) entries now survive a call, both ways (proved against a real server).
- **Errors**, old to new: `Status(code, message)` is `CallError::Status`; `ConnectionFailed` and `Unavailable` are
  `CallError::Transport` with `Unavailable`; `Timeout` is `CallError::Status` with `DeadlineExceeded` (the engine
  reports a passed deadline as a status); `Cancelled` is `CallError::Transport` with `Cancelled`; `Internal` is
  `Encode`, `Decode` or a `Transport` failure with its own code. A path or metadata key that is not valid is refused
  when the `MethodPath` or `MetadataEntry` is created, not at the call.
- **Builder:** `with_max_message_bytes` is `with_max_decoding_message_size` and `with_max_encoding_message_size`
  (a file's `max_message_bytes` sets both: the old client capped responses only); `with_compression(mode)` is
  `with_send_compression` and `with_accept_compression` (an encoding the client did not accept is refused);
  `with_mtls(MtlsConfig)` and `with_root_ca(path)` are `with_tls(TlsSettings)` (bytes; `TlsSettingsResolver::resolve`
  reads PEM files, and a file that cannot be used fails with `UnusableTlsFile` naming its path); a file configuration's
  `mtls` paths are read by `build`. `with_prost_codec` and `DisabledFeatureOptions` are removed.
- **Interceptors:** `before_call(&mut dyn OutboundCall) -> Result<(), CallError>` and `after_call(&InboundReply)`. An
  interceptor adds a header with `OutboundCall::add_header`. It can no longer rewrite a response body.
- **Retry, breaker and resilient** are generic over `Transport + RawTransport` and implement both. Retry classifies by
  `StatusCode` and reads the retry-after hint from the trailers. Their old clients did not forward the three
  streaming shapes and answered `Unimplemented`; they forward them now.
- **Behaviour** (details and decisions in `docs/behaviour-comparison.md`): an oversize response is `Status(OutOfRange)`;
  a unary response with none or several messages is `Status(Unimplemented)`; streams run as they arrive instead of
  being buffered whole; default trust for an `https` endpoint with no CA is the platform's root store, and `rustls` uses
  `ring` through `tonic`; a balanced client holds one client per backend, built once.

### Added

- `OutboundCall` (an interface: path, timeout, headers, `add_header`) and `InboundReply`: what an interceptor sees
  and how it adds a header. An interceptor crate depends on the domain crate only.
- Two views of a client, each bound to an interface of the domain: `GrpcCancellationFactory` (a `CancellationSignal`
  ends a call and every message read from a stream, with `Cancelled`; `TokenCancellation` adapts a `tokio-util`
  `CancellationToken`; dropping a call's future cancels it on the server without it) and `GrpcCallerIdentityFactory`
  (a `CallerIdentity` as the headers `x-trace-id`, `x-tenant-id`, `authorization: Bearer <token>` when it holds a token
  to send, and the further headers it lists; `SecurityContextIdentity` adapts a `SecurityContext`, which
  `grpc-egress-transport-std` re-exports; the domain crate no longer depends on `edge-security-runtime`).
- `HealthCheck`, `HealthCheckRequest`, `HealthCheckResponse`, `HealthStatus`: the `grpc.health.v1.Health/Check` call
  (the old `health_check` opened a TCP connection and said nothing about the service).
- `TlsSettingsResolver`; `scripts/mutation/` (one runner, `mutants.json`) replaces the six `mutation_proof_*.py`.
- The composition-root example has nine scenarios: the six of before, and one for each ruling (a generic client, dropping
  and token cancellation against a real `tonic` server, the security context).

### Removed

`GrpcEgress`, `GrpcRequest`, `GrpcResponse`, `GrpcEgressError`, `GrpcEgressResult`, `GrpcStatusCode`, `CallStreamRequest`,
`CallUnaryWithContextRequest`, `GrpcMessageStreamResponse`, `HealthCheckRequest` (old), `AfterCallRequest`,
`GrpcRequestFactory`, `GrpcRequestBuilder`, `Conversions`, `CompressionModeProvider`, `GrpcEgressProstCodec`,
`ProstUnaryCaller` and the `prost` feature, `call_stream`, and the dependencies only the hand-written client used.

### Fixed

- `PeerServer` of `grpc-egress-test-support` sends the status of a call that carried a message in the trailers, as a
  server does; it sent it in the headers, which a client reads as a response with no message.

### Migration guide: before and after, from the example

`CompositionRoot::assemble` is unchanged. A call through any client is now generic over the contract's trait, never a
`dyn` (the difference of `composition_root.rs` between v0.8.0 and now):

```diff
+use proto_pattern::{CallOptions, CallRequest, MethodPath, RawTransport};
@@
         Ok(builder)
     }
+
+    /// A call made through any client, bare or wrapped: generic over the contract's trait, never a `dyn`.
+    pub async fn ping<T: RawTransport>(client: &T, body: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
+        let path = MethodPath::create("/pkg.Service/Method")?;
+        let request = CallRequest::create(body.to_vec(), CallOptions::default());
+        Ok(client.call_raw(path, request).await?.into_message())
+    }
```

A request and a call, before and after:

```diff
-fn request() -> GrpcRequest {
-    GrpcRequest::new("pkg.Service/Method", b"ping".to_vec(), Duration::from_secs(5))
-}
-let response = client.call_unary(request()).await?;
-assert_eq!(response.body, b"pong");
+let path = MethodPath::create("/pkg.Service/Method")?;
+let request = CallRequest::create(b"ping".to_vec(), CallOptions::create(Some(Duration::from_secs(5)), Metadata::default()));
+let response = client.call_raw(path, request).await?;
+assert_eq!(response.get_message().as_slice(), b"pong");
```

The options, before and after:

```diff
-    .with_max_message_bytes(MESSAGE_CAP_BYTES)
-    .with_compression(CompressionMode::Gzip)
-    .with_mtls(MtlsConfig::with_root_ca(cert, key, ca))
+    .with_max_decoding_message_size(MESSAGE_CAP_BYTES)
+    .with_send_compression(Compression::Gzip)
+    .with_accept_compression(Compression::Gzip)
+    .with_tls(TlsSettingsResolver::resolve(Some(&mtls), mtls.ca_pem_path.as_deref())?)
```

The failures, before and after:

```diff
-Err(GrpcEgressError::Timeout(_))                                  // a passed deadline
-Err(GrpcEgressError::Status(GrpcStatusCode::ResourceExhausted, _)) // a response over the cap
-Err(GrpcEgressError::Unavailable(_))                              // an open breaker
+Err(CallError::Status { code: StatusCode::DeadlineExceeded, .. })
+Err(CallError::Status { code: StatusCode::OutOfRange, .. })
+Err(CallError::Transport { code: StatusCode::Unavailable, .. })
```

## [0.8.0] - 2026-10-16

The restructuring of issues #16 and #17. Breaking throughout: package names, crate layout, the way a
client is built.

### Changed (breaking)

- **Domain first.** Every theme is a pair of crates under `scm/main/`: a domain crate
  `grpc-egress-<theme>` (traits, plain data, errors; no implementation) and an implementation crate
  `grpc-egress-<theme>-std` (`std/`, `spi/` for a library boundary, `saf/` for the factory). Themes:
  `transport`, `retry`, `breaker`, `resilient`, `auth-bearer`. The old packages
  `edge-transport-grpc-egress-*` are gone; `scripts/rename_crate.py` renames a package and every
  reference to it. ADR-006.
- **One way to build a client.** `GrpcEgressBuilder` (`with_endpoint`, `with_plaintext_allowed`,
  `with_request_timeout`, `with_max_message_bytes`, `with_compression`, `with_mtls`, `with_root_ca`,
  `with_keep_alive`, `with_loadbalancer`, `with_prost_codec`, `with_interceptor`, `with_trace_context`,
  `with_channel_config`, `build`) replaces `create_transport_from_config`,
  `create_tonic_client_from_config`, `create_lb_transport_from_config`,
  `create_prost_transport_from_config`, `TransportConstruction`, `GrpcEgressFactory` and
  `ProcessorFactory`. `build()` validates and names every option of a refusal.
- **One factory per theme, no `dyn`.** `GrpcRetryFactory`, `GrpcBreakerFactory`,
  `GrpcResilientFactory` and the transport's `GrpcEgressInterceptorFactory` (also implemented by
  `StdBearerInterceptorFactory`) take a `Build*Request` and return the product, never a trait object. The
  client types are `pub(crate)`. The breaker's product is also a `BreakerObservable`. ADR-007.
- `GrpcRetryFacade`, `GrpcBreakerFacade`, `GrpcResilientFacade`, the `*Svc` marker types and the
  `Processor`, `Validator`, `ConfigBuilderProvider`, `RetryDecorator`, `RetryInspector`,
  `BackoffScheduler` and `BreakerDecorator` traits with their implementations, DTOs and factories are
  removed: none was reachable from a real client.
- `JwtClaimsBuilder::sub` is renamed `subject`. `BearerEgressInterceptor` is no longer public; use
  `StdBearerInterceptorFactory::new(config).build(BuildGrpcEgressInterceptorRequest)`.
- The resilience port and its stub are removed from the transport (the stub reported a permanently closed
  circuit and had no consumer).

### Added

- Options that reach the client for real: `with_mtls` and `with_root_ca` (fixes #13), `with_keep_alive`,
  message compression (gzip, zstd) in both directions, a request timeout no call outlasts,
  `with_loadbalancer` (round robin and weighted, a dead backend avoided), `with_prost_codec` (without the
  Cargo feature `prost`, `build()` returns `DisabledFeatureOptions`).
- `GrpcChannelConfigError` variants `ConflictingOptions`, `InvalidOptionValue` and
  `DisabledFeatureOptions`.
- `grpc-egress-test-support`: an in-process peer (echo, reply, delay, compressed reply, status reply, raw
  reply, TLS and mTLS), a freezable proxy and shared fixtures.
- `scm/main/examples/composition-root`: what a composition root does with these crates, seven scenarios,
  tested; `scripts/mutation_proof_*.py` prove each option's tests fail when the option is removed.
- `scripts/check_layout.py` (run by the pre-commit hook): crate location and names, domain/implementation
  pairs, themes mirrored in every layer, declarations only in `lib.rs`/`mod.rs`, DTOs and errors in their
  folders, no `types/` or `core/`, no `adapter` in a name, no workspace dependency table, every crate a
  workspace member, no `#[allow]`, no test code in `src/`.

### Removed

- All `#[allow(...)]` attributes (tests return `Result` and use `?`), all inline `#[cfg(test)]` modules
  (tests live in `tests/`), the workspace `[workspace.dependencies]` table, and about 300 test attributes
  that tested only coverage stubs or removed code: every name is in `docs/removed-tests.md`
  (generated by `scripts/list_removed_tests.py`).

## [0.7.0] - 2026-08-12

### Changed (breaking)

- The transport no longer knows retry or breaker (ADR-004): `GrpcChannelConfig.resilience` and the types
  that served only it are removed, and the composing facade is retired; composition lives in `resilient`
  and the composition root.
- `GrpcChannelConfigError::Config` is removed (dead once resilience validation left the transport).

### Fixed

- Two stale doc references left by earlier refactors.

## [0.6.2] - 2026-08-10

### Changed

- ADR-004 amended: one composer, the composing facade retired.

## [0.6.1] - 2026-08-07

### Changed

- The security layer is taken from `edge-security-runtime` directly instead of through
  `edge-application`.

## [0.6.0] - 2026-08-07

### Changed (breaking)

- `retry` and `breaker` retargeted to the shared crates `edge-transport-retry` and
  `edge-transport-breaker` (the backoff and jitter core, the breaker state machine and their policy
  types moved there; ADR-003).
- Resilience configuration removed from the transport primitive (ADR-004).

### Changed

- The `breaker` and `retry` examples now prove a real invocation, not only construction.
