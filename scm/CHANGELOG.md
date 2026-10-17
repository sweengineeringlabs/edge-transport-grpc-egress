# Changelog

All notable changes to the workspace are documented here. The per-crate `CHANGELOG.md` files of the
`-std` crates hold the history of their own crate before 0.8.0.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and the workspace
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). A tag is cut only on the owner's
word.

## [Unreleased]

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
- **Interceptors:** `before_call(&mut OutboundCall) -> Result<(), CallError>` and `after_call(&InboundReply)`. An
  interceptor adds a header with `OutboundCallHeaders::add_header`. It can no longer rewrite a response body.
- **Retry, breaker and resilient** are generic over `Transport + RawTransport` and implement both. Retry classifies by
  `StatusCode` and reads the retry-after hint from the trailers. Their old clients did not forward the three
  streaming shapes and answered `Unimplemented`; they forward them now.
- **Behaviour** (details and decisions in `docs/behaviour-comparison.md`): an oversize response is `Status(OutOfRange)`;
  a unary response with none or several messages is `Status(Unimplemented)`; streams run as they arrive instead of
  being buffered whole; default trust for an `https` endpoint with no CA is the platform's root store, and `rustls` uses
  `ring` through `tonic`; a balanced client holds one client per backend, built once.

### Added

- `OutboundCall`, `InboundReply`, `OutboundCallHeaders`: what an interceptor sees and how it adds a header.
- Two views of a client: `GrpcCancellationFactory` (a `CancellationToken` ends a call and every message read from a
  stream, with `Cancelled`; dropping a call's future cancels it on the server without it) and
  `GrpcSecurityContextFactory` (a `SecurityContext` as the headers `x-trace-id`, `x-tenant-id`, `authorization: Bearer
  <token>` when authenticated, and its metadata).
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
