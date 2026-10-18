# ADR-010: Programming over the interface: nothing new is defined in `std/` or `spi/`

**Status:** Accepted (the owner's rule, 2026-10-18)
**Date:** 2026-10-18
**Tracking:** [#60](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/60) (follow-up of [#17](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/17) and [#16](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/16))
**See also:** [ADR-006](ADR-006-domain-first.md) (the domain crate comes first; amended by this record), [ADR-007](ADR-007-decorators-wrap-a-built-client.md)

---

## Context

The owner's rule, verbatim: "Only pub(crate). No new definitions in spi/std. That is the definition of programming
over the interface." ADR-006 made the implementation types `pub(crate)` and `Std*`-named, but nothing enforced
that an implementation crate defines nothing new. Measured on `main` (6b6559b), in the five `*-std` crates:
7 traits declared in `std/`, 28 inherent `impl` blocks and 65 `pub` / `pub(crate)` functions in `std/` and `spi/`. Each is a
definition outside the domain crate, or an operation that no domain trait carries.

A second rule followed (2026-10-18): `std/` is the standard implementation, defaults only, and names no third-party
crate; `spi/` is the only layer that does.

## Decision

### The layers

| Layer | What it is | May depend on | May define |
|---|---|---|---|
| `api/` (the domain crate) | the interface: traits, dto, vo, error, by theme | only what its interface types need; never an engine library (`tonic`, `hyper`, `hyper-util`, `hyper-rustls`, `rustls`, `tower`, `h2`, `prost`, the load balancer, a `tokio` runtime) | everything another module calls |
| `std/` | the standard implementation: defaults only | the domain crate, `core`, `std`, `alloc`; no third-party crate | nothing new: implementations of domain traits on `pub(crate)` types, and private helper functions |
| `spi/` | the service provider implementation: the only layer with third-party dependencies | any library | nothing new: implementations of domain traits on `pub(crate)` types, and private helper functions |
| `saf/` | the factories | the domain crate and the two layers above | one public type per factory, implementing a domain factory trait; no logic, no re-exports |

### What is allowed in `std/` and `spi/`

- Implementations of domain traits, and of standard-library traits (`From`, `TryFrom`, `Default`, `Display`, `Drop`) on `pub(crate)` types.
- Module-private helper functions (no `pub`, no `pub(crate)`), also inside an inherent `impl` block.
- Nothing else: no new trait, no inherent associated function visible outside its own module. A constructor is a
  `From` / `TryFrom` / `Default` impl or a domain factory trait method; an operation is a domain trait method.
- In `std/` additionally: no third-party crate. Code that needs one is in `spi/`, or is a plain default.

### The two enforcing rules

`scripts/check_layout.py` has rule 19, `program_over_the_interface` (a trait declared outside `api/`; a `pub` or
`pub(crate)` function or inherent impl in `std/` or `spi/` that is not a trait impl), and rule 20, `std_names_no_third_party`
(a file under `std/` that names a crate other than the domain crate, `core`, `std`, `alloc` and the crate itself).
The ingress and the server svc copy both with identical wording.

### Decisions the rule forced

- **Orphan rule.** A domain trait cannot be implemented in a `-std` crate for a domain type (both foreign). So an
  extension trait on a data type (`GrpcChannelConfigFactory`, `MtlsConfigFactory`, the `*ConfigProvider`s) becomes a
  domain trait whose implementer is a `pub(crate)` type of `std/` or `spi/`, with a factory trait in the domain and a
  public factory type in `saf/`. The data types stay plain data. Where a trait method is named `from_*` and takes
  `&self`, clippy's `wrong_self_convention` forbids the name (and `#[allow]` is forbidden by the layout rules), so the
  providers' `from_config` is `parse_config`.
- **The contract through the domain crate.** An implementation of a domain trait names the contract types that the
  trait's signature names (`CallError`, `Metadata`, `Transport`, ...). The domain crates therefore re-export the
  contract crate they are written over (`pub use proto_pattern;`), and `std/` reaches it as
  `grpc_egress_transport::proto_pattern::...`: `std/` depends on the domain crate and nothing else. The alternative,
  letting rule 20 allow `proto_pattern`, would change the rule's wording, which the other repositories copy.
- **Public types of the implementation crates.** A type that has no function other than trait impls and `From` is
  allowed by rule 19 even when it is `pub` (`HealthCheck` and its messages, `TokenCancellation`,
  `SecurityContextIdentity`): a caller has to name it. Every builder, resolver, chain and classifier, which a caller
  uses through a trait, becomes `pub(crate)`.
- **`saf/` factories** keep their `new(..)` constructor that binds the factory's construction value (the endpoint of
  `TonicGrpcEgressBuilderSpiFactory`, the policy of `StdGrpcRetryFactory`), as ADR-006 and issue #16 record. It is
  outside rules 19 and 20; it is the path JustRag (#53) uses.

## The offenders and where each goes (task 1)

### Seven traits declared in `std/`

| Trait | File (at 6b6559b) | What it does | Destination |
|---|---|---|---|
| `GrpcChannelConfigFactory` | `transport-std/src/std/channel/channel_config.rs` | constructor and fluent setters of `GrpcChannelConfig` | methods of the domain trait `GrpcChannelConfigBuilder` (api/channel/traits) with `GrpcChannelConfigBuilderFactory`, `BuildGrpcChannelConfigBuilderRequest` (dto); `new(endpoint)` is `with_endpoint`; `with_request_timeout` is a builder method |
| `MtlsConfigFactory` | `transport-std/src/std/channel/mtls_config.rs` | constructors of `MtlsConfig` | domain trait `MtlsConfigFactory` (api/channel/traits), envelope `BuildMtlsConfigRequest` (dto), product `MtlsConfig` (vo, plain data); `new` and `with_root_ca` are one request with an optional CA |
| `LoadbalancerSettingsProvider` | `transport-std/src/std/loadbalancer/loadbalancer_settings.rs` | reads `LoadbalancerSettings` from TOML | domain trait (api/loadbalancer/traits) + factory trait; error `GrpcChannelConfigError` (error, existing); implemented in `spi/` (`toml`, `serde`) |
| `GrpcRetryConfigProvider` | `retry-std/src/std/grpc_retry_config.rs` | parses and validates `GrpcRetryConfig`, derived durations | domain trait (retry api/traits) + factory trait; error `Error` (existing); implemented in `spi/` (`toml`) |
| `GrpcBreakerConfigProvider` | `breaker-std/src/std/grpc_breaker_config.rs` | parses and validates `GrpcBreakerConfig` | domain trait (breaker api/traits) + factory trait; error `Error`; implemented in `spi/` (`toml`) |
| `ResilienceConfigProvider` | `resilient-std/src/std/resilience_config.rs` | parses, validates and projects `ResilienceConfig` | domain trait (resilient api/traits) + factory trait; error `ResilientTransportError`; implemented in `spi/` (`toml`) |
| `BearerSecretGuard` | `auth-bearer-std/src/std/bearer_secret.rs` | constant-time comparison of `BearerSecret` | domain trait (auth-bearer api/traits) + factory trait; implemented in `spi/` (`subtle`) |

### 28 inherent `impl` blocks and 65 functions

Each block, then each `pub` / `pub(crate)` function in it. A block whose destination says "kept" holds private helpers only,
which rule 19 allows.


| # | Block | File (at 6b6559b) | What it does | Public fns | Destination |
|---|---|---|---|---|---|
| 1 | `BearerEgressInterceptor` | `auth-bearer-std/src/spi/jsonwebtoken/bearer_egress_interceptor.rs:20` | builds the interceptor from its config (`from_config`) | 1 | `From<BearerEgressConfig>` on the `pub(crate)` type (spi) |
| 2 | `BearerEgressInterceptor` | `auth-bearer-std/src/spi/jsonwebtoken/bearer_egress_interceptor.rs:27` | signs the token (`sign_token`) | 0 | kept: private helper |
| 3 | `BearerEgressConfigBuilder` | `auth-bearer-std/src/std/bearer_egress_config_builder.rs:16` | fluent builder of `BearerEgressConfig` | 7 | domain trait `BearerEgressConfigBuilder` + factory trait (api/traits, api/dto); `StdBearerEgressConfigBuilder` (std, `pub(crate)`) implements it; `new` is `Default`; saf `StdBearerEgressConfigBuilderFactory` |
| 4 | `JwtClaimsBuilder` | `auth-bearer-std/src/std/jwt_claims_builder.rs:15` | fluent builder of `JwtClaims` | 7 | domain trait `JwtClaimsBuilder` + factory trait; `StdJwtClaimsBuilder` (std, `pub(crate)`) implements it; `new` is `Default`; saf `StdJwtClaimsBuilderFactory` |
| 5 | `GrpcBreakerClient<T>` | `breaker-std/src/std/breaker_egress.rs:20` | observability and request handling of the breaker client (private helpers) | 0 | kept: private helpers; file moves to spi/ (names `tracing`, `edge-transport-breaker`) |
| 6 | `StdFailureClassifier` | `breaker-std/src/std/failure_classifier.rs:34` | classifies a call result as breaker success or failure (`classify_result`) | 1 | domain trait `FailureClassifier` (api/traits); `StdFailureClassifier` implements it; file moves to spi/ (`tracing`, policy crate) |
| 7 | `GrpcBreakerClient<T>` | `breaker-std/src/std/grpc_breaker_client.rs:33` | constructor of the breaker client (`new`) | 1 | `From<(T, GrpcBreakerConfig)>`; file moves to spi/ (`tokio`, `futures`, `tracing`) |
| 8 | `GrpcRetryClient<T>` | `retry-std/src/std/grpc_retry_client.rs:44` | constructor of the retry client (`new`) | 1 | `From<(T, GrpcRetryConfig)>`; file moves to spi/ |
| 9 | `GrpcRetryConfigBuilder` | `retry-std/src/std/grpc_retry_config_builder.rs:35` | fluent builder of `GrpcRetryConfig` | 10 | domain trait `GrpcRetryConfigBuilder` + factory trait; `StdGrpcRetryConfigBuilder` (std, `pub(crate)`) implements it; `new` is `Default`; saf `StdGrpcRetryConfigBuilderFactory` |
| 10 | `ResourceExhaustedContext` | `retry-std/src/std/resource_exhausted_context.rs:31` | classifies a RESOURCE_EXHAUSTED message (`classify`) | 1 | domain trait `RetryClassifier` (api/traits) carries `classify_resource_exhausted`; the enum moves to api/vo; `StdRetryClassifier` implements it |
| 11 | `RetryDecision` | `retry-std/src/std/retry_decision.rs:43` | per-status retry decision (`should_retry`, `is_rate_limit`, `classify`, `read_retry_after`, `parse_retry_after_hint`) | 5 | domain trait `RetryClassifier`; the enum `RetryDecision` moves to api/vo; `StdRetryClassifier` implements it; saf `StdRetryClassifierFactory` |
| 12 | `GrpcRetryClient<T>` | `retry-std/src/std/retry_transport.rs:147` | retry loop helpers (private) | 0 | kept: private helpers; file moves to spi/ (`tokio`, `tracing`, `edge-transport-retry`) |
| 13 | `EndpointTransport<T>` | `transport-std/src/spi/client/protosvc/endpoint_transport.rs:34` | one endpoint or several backends behind the load balancer (`create`) | 1 | `TryFrom<(Vec<(String, T)>, Option<&LoadbalancerSettings>)>` (spi) |
| 14 | `GrpcEgressBuilderImpl` | `transport-std/src/spi/client/protosvc/grpc_egress_builder.rs:54` | state of the client builder (`new`) | 1 | `From<String>` (spi) |
| 15 | `GrpcEgressBuilderImpl` | `transport-std/src/spi/client/protosvc/grpc_egress_builder.rs:213` | validation and construction helpers (private) | 0 | kept: private helpers |
| 16 | `CancellableResponseStream<S, R, G>` | `transport-std/src/std/cancellation/cancellable_response_stream.rs:18` | response stream ended by a signal (`create`) | 1 | `From<(S, Arc<G>, &str)>`; file moves to spi/ (`tokio`) |
| 17 | `CancellableTransport<'a, T, S>` | `transport-std/src/std/cancellation/cancellable_transport.rs:24` | client view ended by a signal (`create`) | 1 | `From<BuildGrpcCancellationRequest>`; file moves to spi/ (`tokio`) |
| 18 | `TokenCancellation` | `transport-std/src/std/cancellation/token_cancellation.rs:12` | a `CancellationToken` as a signal (`new`) | 1 | `From<CancellationToken>`; file moves to spi/ (`tokio-util`) |
| 19 | `GrpcChannelConfigBuilder` | `transport-std/src/std/channel/channel_config.rs:82` | fluent builder of `GrpcChannelConfig` | 8 | domain trait `GrpcChannelConfigBuilder` + factory trait (api/channel); `StdGrpcChannelConfigBuilder` (std, `pub(crate)`) implements it; `new` is `Default`; saf `StdGrpcChannelConfigBuilderFactory` |
| 20 | `TlsSettingsResolver` | `transport-std/src/std/channel/tls_settings_resolver.rs:16` | reads the PEM files of the TLS options (`resolve`) | 1 | domain trait `TlsSettingsResolver` + factory trait (api/channel), envelope `ResolveTlsSettingsRequest`; `StdTlsSettingsResolver` (std, `pub(crate)`) implements it; saf `StdTlsSettingsResolverFactory` |
| 21 | `MetadataExtender` | `transport-std/src/std/client/metadata_extender.rs:10` | copies metadata with entries added (`append`, `extend`, `copy`) | 3 | domain trait `MetadataExtender` (api/client/traits); `StdMetadataExtender` (std, `pub(crate)`) implements it |
| 22 | `ContextHeaders` | `transport-std/src/std/context/context_headers.rs:8` | headers derived from a caller identity (`derive`) | 1 | `From<&dyn CallerIdentity>` (std) |
| 23 | `ContextTransport<'a, T>` | `transport-std/src/std/context/context_transport.rs:23` | client view that carries identity headers (`create`) | 1 | `From<BuildGrpcCallerIdentityRequest>` (std) |
| 24 | `SecurityContextIdentity<'a>` | `transport-std/src/std/context/security_context_identity.rs:12` | a `SecurityContext` as a caller identity (`new`) | 1 | `From<&SecurityContext>`; file moves to spi/ (`edge-security-runtime`) |
| 25 | `GrpcEgressInterceptorChain` | `transport-std/src/std/interceptor/egress_interceptor_chain.rs:10` | ordered chain of interceptors (`new`, `push`, `len`, `is_empty`, `run_before`, `run_after`) | 6 | domain trait `GrpcEgressInterceptorChain` + factory trait (api/interceptor); `StdGrpcEgressInterceptorChain` (std, `pub(crate)`) implements it; `new` is `Default`; saf `StdGrpcEgressInterceptorChainFactory` |
| 26 | `InterceptedTransport<T>` | `transport-std/src/std/interceptor/intercepted_transport.rs:23` | client that runs a chain around each call (`create`) | 1 | `From<(T, StdGrpcEgressInterceptorChain)>` (std) |
| 27 | `PendingCall<'a>` | `transport-std/src/std/interceptor/pending_call.rs:17` | the call an interceptor edits (`create`, `into_options`) | 2 | `From<(&str, &CallOptions)> for PendingCall` and `From<PendingCall> for CallOptions` (std) |
| 28 | `TraceContextGrpcEgressInterceptor` | `transport-std/src/std/interceptor/trace_context_grpc_egress_interceptor.rs:17` | injects the W3C trace context (`pass_through`, `with_static`) | 2 | `From<TraceContextSource>` (std); type becomes `pub(crate)`, created through `StdGrpcEgressInterceptorFactory` |

| # | Function | File:line | Kind | Destination |
|---|---|---|---|---|
| 1 | `BearerEgressInterceptor::from_config` | `auth-bearer-std/src/spi/jsonwebtoken/bearer_egress_interceptor.rs:22` | pub(crate) | `From<BearerEgressConfig>` on the `pub(crate)` type (spi) |
| 2 | `BearerEgressConfigBuilder::new` | `auth-bearer-std/src/std/bearer_egress_config_builder.rs:18` | pub | domain trait `BearerEgressConfigBuilder` + factory trait (api/traits, api/dto) |
| 3 | `BearerEgressConfigBuilder::secret` | `auth-bearer-std/src/std/bearer_egress_config_builder.rs:23` | pub | domain trait `BearerEgressConfigBuilder` + factory trait (api/traits, api/dto) |
| 4 | `BearerEgressConfigBuilder::issuer` | `auth-bearer-std/src/std/bearer_egress_config_builder.rs:29` | pub | domain trait `BearerEgressConfigBuilder` + factory trait (api/traits, api/dto) |
| 5 | `BearerEgressConfigBuilder::audience` | `auth-bearer-std/src/std/bearer_egress_config_builder.rs:35` | pub | domain trait `BearerEgressConfigBuilder` + factory trait (api/traits, api/dto) |
| 6 | `BearerEgressConfigBuilder::subject` | `auth-bearer-std/src/std/bearer_egress_config_builder.rs:41` | pub | domain trait `BearerEgressConfigBuilder` + factory trait (api/traits, api/dto) |
| 7 | `BearerEgressConfigBuilder::lifetime_seconds` | `auth-bearer-std/src/std/bearer_egress_config_builder.rs:47` | pub | domain trait `BearerEgressConfigBuilder` + factory trait (api/traits, api/dto) |
| 8 | `BearerEgressConfigBuilder::build` | `auth-bearer-std/src/std/bearer_egress_config_builder.rs:55` | pub | domain trait `BearerEgressConfigBuilder` + factory trait (api/traits, api/dto) |
| 9 | `JwtClaimsBuilder::new` | `auth-bearer-std/src/std/jwt_claims_builder.rs:17` | pub | domain trait `JwtClaimsBuilder` + factory trait |
| 10 | `JwtClaimsBuilder::iss` | `auth-bearer-std/src/std/jwt_claims_builder.rs:22` | pub | domain trait `JwtClaimsBuilder` + factory trait |
| 11 | `JwtClaimsBuilder::aud` | `auth-bearer-std/src/std/jwt_claims_builder.rs:28` | pub | domain trait `JwtClaimsBuilder` + factory trait |
| 12 | `JwtClaimsBuilder::subject` | `auth-bearer-std/src/std/jwt_claims_builder.rs:34` | pub | domain trait `JwtClaimsBuilder` + factory trait |
| 13 | `JwtClaimsBuilder::exp` | `auth-bearer-std/src/std/jwt_claims_builder.rs:40` | pub | domain trait `JwtClaimsBuilder` + factory trait |
| 14 | `JwtClaimsBuilder::iat` | `auth-bearer-std/src/std/jwt_claims_builder.rs:46` | pub | domain trait `JwtClaimsBuilder` + factory trait |
| 15 | `JwtClaimsBuilder::build` | `auth-bearer-std/src/std/jwt_claims_builder.rs:54` | pub | domain trait `JwtClaimsBuilder` + factory trait |
| 16 | `StdFailureClassifier::classify_result` | `breaker-std/src/std/failure_classifier.rs:36` | pub(crate) | domain trait `FailureClassifier` (api/traits) |
| 17 | `GrpcBreakerClient<T>::new` | `breaker-std/src/std/grpc_breaker_client.rs:35` | pub(crate) | `From<(T, GrpcBreakerConfig)>` |
| 18 | `GrpcRetryClient<T>::new` | `retry-std/src/std/grpc_retry_client.rs:46` | pub(crate) | `From<(T, GrpcRetryConfig)>` |
| 19 | `GrpcRetryConfigBuilder::new` | `retry-std/src/std/grpc_retry_config_builder.rs:37` | pub | domain trait `GrpcRetryConfigBuilder` + factory trait |
| 20 | `GrpcRetryConfigBuilder::max_attempts` | `retry-std/src/std/grpc_retry_config_builder.rs:52` | pub | domain trait `GrpcRetryConfigBuilder` + factory trait |
| 21 | `GrpcRetryConfigBuilder::initial_backoff_ms` | `retry-std/src/std/grpc_retry_config_builder.rs:58` | pub | domain trait `GrpcRetryConfigBuilder` + factory trait |
| 22 | `GrpcRetryConfigBuilder::backoff_multiplier` | `retry-std/src/std/grpc_retry_config_builder.rs:64` | pub | domain trait `GrpcRetryConfigBuilder` + factory trait |
| 23 | `GrpcRetryConfigBuilder::jitter_factor` | `retry-std/src/std/grpc_retry_config_builder.rs:70` | pub | domain trait `GrpcRetryConfigBuilder` + factory trait |
| 24 | `GrpcRetryConfigBuilder::max_backoff_ms` | `retry-std/src/std/grpc_retry_config_builder.rs:76` | pub | domain trait `GrpcRetryConfigBuilder` + factory trait |
| 25 | `GrpcRetryConfigBuilder::rate_limit_max_attempts` | `retry-std/src/std/grpc_retry_config_builder.rs:82` | pub | domain trait `GrpcRetryConfigBuilder` + factory trait |
| 26 | `GrpcRetryConfigBuilder::rate_limit_initial_backoff_ms` | `retry-std/src/std/grpc_retry_config_builder.rs:88` | pub | domain trait `GrpcRetryConfigBuilder` + factory trait |
| 27 | `GrpcRetryConfigBuilder::rate_limit_max_backoff_ms` | `retry-std/src/std/grpc_retry_config_builder.rs:94` | pub | domain trait `GrpcRetryConfigBuilder` + factory trait |
| 28 | `GrpcRetryConfigBuilder::build` | `retry-std/src/std/grpc_retry_config_builder.rs:102` | pub | domain trait `GrpcRetryConfigBuilder` + factory trait |
| 29 | `ResourceExhaustedContext::classify` | `retry-std/src/std/resource_exhausted_context.rs:33` | pub | domain trait `RetryClassifier` (api/traits) carries `classify_resource_exhausted` |
| 30 | `RetryDecision::should_retry` | `retry-std/src/std/retry_decision.rs:45` | pub | domain trait `RetryClassifier` |
| 31 | `RetryDecision::is_rate_limit` | `retry-std/src/std/retry_decision.rs:50` | pub | domain trait `RetryClassifier` |
| 32 | `RetryDecision::classify` | `retry-std/src/std/retry_decision.rs:73` | pub | domain trait `RetryClassifier` |
| 33 | `RetryDecision::read_retry_after` | `retry-std/src/std/retry_decision.rs:116` | pub | domain trait `RetryClassifier` |
| 34 | `RetryDecision::parse_retry_after_hint` | `retry-std/src/std/retry_decision.rs:146` | pub | domain trait `RetryClassifier` |
| 35 | `EndpointTransport<T>::create` | `transport-std/src/spi/client/protosvc/endpoint_transport.rs:41` | pub(crate) | `TryFrom<(Vec<(String, T)>, Option<&LoadbalancerSettings>)>` (spi) |
| 36 | `GrpcEgressBuilderImpl::new` | `transport-std/src/spi/client/protosvc/grpc_egress_builder.rs:56` | pub(crate) | `From<String>` (spi) |
| 37 | `CancellableResponseStream<S, R, G>::create` | `transport-std/src/std/cancellation/cancellable_response_stream.rs:20` | pub(crate) | `From<(S, Arc<G>, &str)>` |
| 38 | `CancellableTransport<'a, T, S>::create` | `transport-std/src/std/cancellation/cancellable_transport.rs:26` | pub(crate) | `From<BuildGrpcCancellationRequest>` |
| 39 | `TokenCancellation::new` | `transport-std/src/std/cancellation/token_cancellation.rs:14` | pub | `From<CancellationToken>` |
| 40 | `GrpcChannelConfigBuilder::new` | `transport-std/src/std/channel/channel_config.rs:84` | pub | domain trait `GrpcChannelConfigBuilder` + factory trait (api/channel) |
| 41 | `GrpcChannelConfigBuilder::with_endpoint` | `transport-std/src/std/channel/channel_config.rs:91` | pub | domain trait `GrpcChannelConfigBuilder` + factory trait (api/channel) |
| 42 | `GrpcChannelConfigBuilder::allow_plaintext` | `transport-std/src/std/channel/channel_config.rs:96` | pub | domain trait `GrpcChannelConfigBuilder` + factory trait (api/channel) |
| 43 | `GrpcChannelConfigBuilder::with_mtls` | `transport-std/src/std/channel/channel_config.rs:101` | pub | domain trait `GrpcChannelConfigBuilder` + factory trait (api/channel) |
| 44 | `GrpcChannelConfigBuilder::with_keep_alive` | `transport-std/src/std/channel/channel_config.rs:106` | pub | domain trait `GrpcChannelConfigBuilder` + factory trait (api/channel) |
| 45 | `GrpcChannelConfigBuilder::with_max_message_bytes` | `transport-std/src/std/channel/channel_config.rs:111` | pub | domain trait `GrpcChannelConfigBuilder` + factory trait (api/channel) |
| 46 | `GrpcChannelConfigBuilder::with_compression` | `transport-std/src/std/channel/channel_config.rs:116` | pub | domain trait `GrpcChannelConfigBuilder` + factory trait (api/channel) |
| 47 | `GrpcChannelConfigBuilder::build` | `transport-std/src/std/channel/channel_config.rs:122` | pub | domain trait `GrpcChannelConfigBuilder` + factory trait (api/channel) |
| 48 | `TlsSettingsResolver::resolve` | `transport-std/src/std/channel/tls_settings_resolver.rs:22` | pub | domain trait `TlsSettingsResolver` + factory trait (api/channel), envelope `ResolveTlsSettingsRequest` |
| 49 | `MetadataExtender::append` | `transport-std/src/std/client/metadata_extender.rs:16` | pub(crate) | domain trait `MetadataExtender` (api/client/traits) |
| 50 | `MetadataExtender::extend` | `transport-std/src/std/client/metadata_extender.rs:32` | pub(crate) | domain trait `MetadataExtender` (api/client/traits) |
| 51 | `MetadataExtender::copy` | `transport-std/src/std/client/metadata_extender.rs:47` | pub(crate) | domain trait `MetadataExtender` (api/client/traits) |
| 52 | `ContextHeaders::derive` | `transport-std/src/std/context/context_headers.rs:11` | pub(crate) | `From<&dyn CallerIdentity>` (std) |
| 53 | `ContextTransport<'a, T>::create` | `transport-std/src/std/context/context_transport.rs:25` | pub(crate) | `From<BuildGrpcCallerIdentityRequest>` (std) |
| 54 | `SecurityContextIdentity<'a>::new` | `transport-std/src/std/context/security_context_identity.rs:14` | pub | `From<&SecurityContext>` |
| 55 | `GrpcEgressInterceptorChain::new` | `transport-std/src/std/interceptor/egress_interceptor_chain.rs:12` | pub | domain trait `GrpcEgressInterceptorChain` + factory trait (api/interceptor) |
| 56 | `GrpcEgressInterceptorChain::push` | `transport-std/src/std/interceptor/egress_interceptor_chain.rs:19` | pub | domain trait `GrpcEgressInterceptorChain` + factory trait (api/interceptor) |
| 57 | `GrpcEgressInterceptorChain::len` | `transport-std/src/std/interceptor/egress_interceptor_chain.rs:25` | pub | domain trait `GrpcEgressInterceptorChain` + factory trait (api/interceptor) |
| 58 | `GrpcEgressInterceptorChain::is_empty` | `transport-std/src/std/interceptor/egress_interceptor_chain.rs:30` | pub | domain trait `GrpcEgressInterceptorChain` + factory trait (api/interceptor) |
| 59 | `GrpcEgressInterceptorChain::run_before` | `transport-std/src/std/interceptor/egress_interceptor_chain.rs:39` | pub | domain trait `GrpcEgressInterceptorChain` + factory trait (api/interceptor) |
| 60 | `GrpcEgressInterceptorChain::run_after` | `transport-std/src/std/interceptor/egress_interceptor_chain.rs:51` | pub | domain trait `GrpcEgressInterceptorChain` + factory trait (api/interceptor) |
| 61 | `InterceptedTransport<T>::create` | `transport-std/src/std/interceptor/intercepted_transport.rs:25` | pub(crate) | `From<(T, StdGrpcEgressInterceptorChain)>` (std) |
| 62 | `PendingCall<'a>::create` | `transport-std/src/std/interceptor/pending_call.rs:19` | pub(crate) | `From<(&str, &CallOptions)> for PendingCall` and `From<PendingCall> for CallOptions` (std) |
| 63 | `PendingCall<'a>::into_options` | `transport-std/src/std/interceptor/pending_call.rs:28` | pub(crate) | `From<(&str, &CallOptions)> for PendingCall` and `From<PendingCall> for CallOptions` (std) |
| 64 | `TraceContextGrpcEgressInterceptor::pass_through` | `transport-std/src/std/interceptor/trace_context_grpc_egress_interceptor.rs:19` | pub | `From<TraceContextSource>` (std) |
| 65 | `TraceContextGrpcEgressInterceptor::with_static` | `transport-std/src/std/interceptor/trace_context_grpc_egress_interceptor.rs:26` | pub | `From<TraceContextSource>` (std) |

### Files of `std/` that name a third-party crate (task 4)

The audit read every file under `std/` of the five crates (`proto_pattern` and the domain crates are not third-party
for this table: see "The contract through the domain crate").

| File (at 6b6559b) | Names | What it does | Destination |
|---|---|---|---|
| `transport-std/src/std/cancellation/cancellable_transport.rs` | `tokio` | view whose calls end when a signal fires (`tokio::select!`) | move to `spi/cancellation/signal/` |
| `transport-std/src/std/cancellation/cancellable_response_stream.rs` | `tokio` | stream ended by a signal | move to `spi/cancellation/signal/` |
| `transport-std/src/std/cancellation/token_cancellation.rs` | `tokio_util` | a `CancellationToken` as a signal | move to `spi/cancellation/signal/`; `From<CancellationToken>` |
| `transport-std/src/std/context/security_context_identity.rs` | `edge_security_runtime` | a `SecurityContext` as a caller identity | move to `spi/context/security/`; `From<&SecurityContext>` |
| `transport-std/src/std/loadbalancer/loadbalancer_settings.rs` | `serde`, `toml` | reads load balancer settings | move to `spi/loadbalancer/settings/` |
| `transport-std/src/std/client/health_check.rs`, `dto/health_check_request.rs`, `dto/health_check_response.rs`, `vo/health_status.rs` | `prost` | the health-check call and its messages | move to `spi/client/health/` |
| `retry-std/src/std/retry_transport.rs`, `grpc_retry_client.rs` | `tokio`, `tracing`, `edge_transport_retry` | the retry decorator | move to `spi/` |
| `retry-std/src/std/grpc_retry_config.rs` | `toml` | parses the retry policy | move to `spi/` |
| `breaker-std/src/std/grpc_breaker_client.rs`, `breaker_egress.rs`, `failure_classifier.rs` | `tokio`, `futures`, `tracing`, `edge_transport_breaker_policy`, `edge_transport_breaker` | the breaker decorator | move to `spi/` |
| `breaker-std/src/std/grpc_breaker_config.rs` | `toml` | parses the breaker policy | move to `spi/` |
| `resilient-std/src/std/resilience_config.rs` | `toml` | parses the resilience policy | move to `spi/` |
| `auth-bearer-std/src/std/bearer_secret.rs` | `subtle` | constant-time comparison | move to `spi/` |
| `auth-bearer-std/src/std/jwt_claims_builder.rs` | `swe_edge_security` | builds `JwtClaims` | stays in `std/`: the domain crate re-exports `JwtClaims`, so the file names the domain crate |

After the move `breaker-std` and `resilient-std` have no `std/` folder: everything they hold names a third-party crate or
is created in `saf/`. That is allowed; a crate has the layers it needs.

## Rejected alternatives

- **Keep the extension traits in `std/` and only make them `pub(crate)`.** A `pub(crate)` trait is not callable by a
  caller, and the rule says everything another module calls is defined in the domain crate.
- **Implement the moved domain traits for the data types in the domain crate.** It makes the domain crate carry
  implementation (ADR-006 rejected it), and puts logic such as TOML parsing and validation next to the data.
- **Make a constructor `pub(crate)`.** The rule forbids the function, not only its visibility: a constructor is `From`,
  `TryFrom`, `Default` or a factory trait method.
- **A struct literal in place of `MtlsConfig::new`.** The fields are public, so it works, but it removes a way of
  building the value that the interface offered; the factory trait keeps the operation.
- **Let rule 20 allow `proto_pattern`.** It would change the wording of a rule that three repositories copy; the domain
  crate re-exports the contract instead.
- **Keep third-party code in `std/` behind a feature.** `std/` is defaults only; a feature does not change what the
  files name.

## Consequences

- Breaking for a caller of the old paths; each change is in `scm/CHANGELOG.md` under "Breaking (layout)" with the old
  path and the new way.
- `TonicGrpcEgressBuilderSpiFactory::new(endpoint)` and the builder traits (the path JustRag #53 uses) are unchanged.
- ADR-006's table of extension traits is superseded by the table above.
- The test counts do not fall; tests that changed their call syntax keep their names, and the removed ones are in
  `scm/docs/removed-tests.md`.
