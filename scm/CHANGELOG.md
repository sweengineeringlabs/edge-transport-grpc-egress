# Changelog

All notable changes to the workspace are documented here. The per-crate `CHANGELOG.md` files of the
`-std` crates hold the history of their own crate before 0.8.0.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and the workspace
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). A tag is cut only on the owner's
word.

## [Unreleased]

Issue 18, stage 1: the engine of the client is `proto-svc`. Breaking in what a failure reports (see
`docs/behaviour-comparison.md`, which accounts for every test of v0.8.0 that behaved differently against the new
engine, with the decision for each); the public API of the builder is unchanged.

### Changed (breaking)

- **The client calls through `proto-svc`** (`proto-pattern` 0.9.2 `RawTransport`, `proto-svc` 0.3.x `tonic`): the
  hand-written `hyper` client, its frame codec and its TLS connector are removed. `rust-version` is 1.99.
- A call that passed its deadline is `Status(DeadlineExceeded, _)`, not `Timeout`.
- A response over `max_message_bytes` is `Status(OutOfRange, _)` (`tonic`'s code), not `Status(ResourceExhausted, _)`.
- A unary response with no message, or with several, is `Status(Unimplemented, _)`.
- A response compressed with an encoding the client did not advertise is refused; `with_compression` advertises it.
- Default trust for an `https` endpoint with no CA is the platform's root store (was the bundled web PKI roots);
  `rustls` uses `ring` through `tonic` (was `aws-lc-rs`; ADR-005).
- Streams run as they arrive (they were buffered whole); a failure while reading is an item of the stream.
- A balanced client holds one client per backend, built once.
- **Removed:** `GrpcEgressBuilder::with_prost_codec` and `GrpcChannelConfigError::DisabledFeatureOptions`. The
  engine always sends `application/grpc`, so the option could change nothing on the wire. The composition-root
  example has six scenarios; the prost one went with the option. The `prost` feature of the implementation crate
  still exposes `ProstUnaryCaller`.

### Fixed

- `PeerServer` of `grpc-egress-test-support` sends the status of a call that carried a message in the trailers, as a
  server does; it sent it in the headers, which a client reads as a response with no message.

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
