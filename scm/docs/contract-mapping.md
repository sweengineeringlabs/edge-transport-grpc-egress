# Contract mapping: every public item of v0.8.0 against `proto-pattern` and `proto-svc`

Task 1 of `edge-transport-grpc-egress#18`, as executed. Checked against v0.8.0 (the `GrpcEgress` API), `proto-pattern`
0.10.0 (`Transport`, `RawTransport`, `MethodPath`, `DualTransportFactory`) and `proto-svc` 0.4.0
(`TonicTransportSpiFactory`). Each row is **replaced** (by a contract item), **kept** (with the reason) or **removed**
(with the reason). The design is ADR-009; the behaviour differences found while moving are in `behaviour-comparison.md`.

## Current dependency API

The workspace now pins `proto-pattern` v0.12.0 and `proto-svc` v0.4.2.
Raw calls accept a single operation-specific request built with `CallRaw*RequestBuilder`;
streaming typed calls use `Call*StreamRequest`. Their response envelopes expose
`into_response()` or `into_stream()`. Stream and metadata queries likewise take request
DTOs and return fallible response envelopes. Typed unary `CallRequest` and `CallResponse`
use `new`. Engine configuration starts at `tonic_dual_builder()` and terminates with
`DualTransportBuilder::build`. The historical mapping below records the original migration.

## Issue 61: current cross-cutting ownership

| Concern | Current owner and mapping |
|---|---|
| Typed/raw pre-call identity and options | Pattern `CallIdentity`, `CallKind`, `CallStart`, and `TransportInterceptor::before_call`; `InterceptedTransport` delegates through a per-call adapter. |
| Lifecycle notifications | Pattern `CallEvent`; the adapter emits `Started`, stream `Opened`/`MessageRead`, and one terminal `Completed`/`Failed`/`Cancelled`, then forwards events to the egress chain in registration order. Metadata is converted to the engine's concrete `Metadata` only for this synchronous callback; conversion failure suppresses observation and does not change the RPC result. |
| Egress runtime registration | `GrpcEgressInterceptor` and its ordered `Arc<dyn ...>` chain stay in egress because Pattern's generic callback is not object-safe. Its default `on_event` is an object-safe bridge over Pattern's existing `CallEvent` type and tonic metadata; it does not define a second event model. |
| Fallible reply policy | `after_call` stays egress-owned. It runs only after successful unary and client-streaming calls and may still reject the reply. Pattern events remain observational; egress reports `Failed` when its reply policy rejects a result. |
| Retry, breaker, resilience, authentication, trace policy, load-balancing policy, and validation | Remain in their current domain/security/consumer owners; none duplicate Pattern call lifecycle or service transport construction. ADR-007 continues to order egress interceptors inside retry and the breaker outside retry. |
| Builder options | `GrpcEgressBuilder` remains the consumer facade and keeps current per-option precedence, file conversion, and load-balancer behavior. Its portable endpoint/TLS/timeout/limit/compression values are applied through Pattern's builder contract by `proto-svc`; HTTP/2 keep-alive remains a service option. Egress retains the collection state needed to replay the same settings for each load-balanced backend and preserve `with_channel_config` overlays. |

The egress-owned adapter types provide runtime registration and reply-refusal behavior that neither Pattern nor the service owns. No separate egress lifecycle event enum or portable transport configuration type is introduced.

## The finding that shaped the rest

`GrpcEgress` was a byte-oriented call over a path given as a string: `GrpcRequest { method: String, body: Vec<u8>, .. }`.
That is `RawTransport`, not `Transport`, whose path is a constant of a type. `proto-pattern#19` and `proto-svc#4` exist for this
reason. A caller that calls by path keeps its call shape and moves to `RawTransport`; a caller that has a `Method` type
uses `Transport`; both come from one client (`DualTransportFactory`, one engine).

## Port and call types (`grpc-egress-transport`, `api/client`)

| Item | Disposition | Contract item or reason |
|---|---|---|
| `GrpcEgress` | replaced | `Transport` + `RawTransport`; every consumer is a generic bound, no `dyn` and no `BoxFuture` (ADR-008 ruling 1) |
| `GrpcEgress::call_unary` | replaced | `RawTransport::call_raw` / `Transport::call` |
| `call_server_stream`, `call_client_stream`, `call_bidi_stream` | replaced | `call_raw_server_stream`, `call_raw_client_stream`, `call_raw_bidi_stream` / the typed three |
| `call_stream` (default `Unimplemented`) | removed | every real shape has its own method; no consumer in this repository or its dependents used it (checked) |
| `call_unary_with_context` | replaced | the caller-identity view (`GrpcCallerIdentityFactory`, ADR-009 section 4) |
| `health_check` | replaced | `HealthCheck`, the `grpc.health.v1.Health/Check` call (`grpc-egress-transport-std`); the old one opened a TCP connection and said nothing about the service |
| `GrpcRequest` | replaced | `MethodPath` + `CallRequest<Vec<u8>>` + `CallOptions`; its `cancellation` token is the cancellation view (`GrpcCancellationFactory`) |
| `GrpcResponse` | replaced | `CallResponse<Vec<u8>, M>` (headers and trailers apart) |
| `GrpcMessageStreamResponse`, `CallStreamRequest` | replaced | `impl ResponseStream<Vec<u8>>` and `impl MessageStream<Vec<u8>>`: typed, no boxing |
| `CallUnaryWithContextRequest`, `HealthCheckRequest` (old) | replaced | with the views above; the health messages are `HealthCheckRequest`/`HealthCheckResponse` of the `grpc.health.v1` protocol |
| `GrpcEgressError`, `GrpcEgressResult` | replaced | `CallError` (a `StatusCode` on every failure), per-variant map in ADR-009 section 6, each proved against a real server |
| `GrpcStatusCode` | replaced | `StatusCode` |
| `HashMap<String, String>` metadata | replaced | `Metadata`: many values per key and binary entries, both proved against a real server |
| `GrpcEgressBuilder`, `GrpcEgressBuilderFactory`, `BuildGrpcEgressBuilderRequest` | kept | the composition seam of this crate; `build` now returns `impl Transport + RawTransport` |
| `GrpcEgressProstCodec`, `ProstUnaryCaller`, the `prost` feature | removed | typed calls are `Transport::call::<M>` with the caller's `Payload` |
| `GrpcRequestFactory`, `GrpcRequestBuilder`, `Conversions`, `CompressionModeProvider` | removed | they built or converted the removed types |

## Channel configuration (`api/channel`)

| Item | Disposition | Contract item or reason |
|---|---|---|
| `GrpcChannelConfig`, `GrpcChannelConfigError` | kept | the loaded configuration (TOML) and the refusals of `build` are this crate's |
| `MtlsConfig` (PEM paths) | kept, for the file only | the contract reads no path and has no serde; the builder takes `TlsSettings` (bytes), and `TlsSettingsResolver::resolve` reads the files named here, failing with `UnusableTlsFile` naming the path |
| `CompressionMode` | kept, for the file only | the contract's `Compression` has no serde and no "none", and a configuration file needs both; the builder takes `Compression` (`with_send_compression`, `with_accept_compression`) |
| `KeepAliveConfig` | kept | the contract has no keep-alive type (it is an engine option); the builder maps it to the engine's four options |
| `max_message_bytes`, `request_timeout_secs` (fields) | kept | file fields; `max_message_bytes` sets both the decoding and the encoding limit |
| `DEFAULT_MAX_MESSAGE_BYTES`, `DEFAULT_REQUEST_TIMEOUT_SECS`, `GrpcChannelConfigBuilder`, `GrpcChannelConfigFactory`, `MtlsConfigFactory` | kept | build this crate's configuration |

## Interceptors, views and load balancing

| Item | Disposition | Contract item or reason |
|---|---|---|
| `GrpcEgressInterceptor` | kept, new signature | the contract has no interceptor concept; `before_call(&mut OutboundCall)` / `after_call(&InboundReply)` over `Metadata`, errors are `CallError` |
| `AfterCallRequest` | replaced | `InboundReply` (path, headers, trailers); an interceptor can no longer rewrite the response body: the typed layer has no bytes, and no interceptor here did |
| `GrpcEgressInterceptorFactory`, `BuildGrpcEgressInterceptorRequest`, `GrpcEgressInterceptorChain`, `TraceContextGrpcEgressInterceptor`, `TraceContextSource` | kept | W3C trace headers and the ordered chain are not in the contract |
| `OutboundCall` (interface), `InboundReply` | new | what an interceptor sees, and how it adds a header |
| cancellation view (`GrpcCancellationFactory`, `BuildGrpcCancellationRequest`) | new | keeps the cancellation capability behind `CancellationSignal` (`TokenCancellation` adapts a token); dropping a call's future also cancels it |
| caller-identity view (`GrpcCallerIdentityFactory`, `BuildGrpcCallerIdentityRequest`) | new | keeps the security-context capability as headers of the call, behind `CallerIdentity` (`SecurityContextIdentity` adapts a `SecurityContext`) |
| `BackendTarget`, `BalancingStrategy`, `LoadbalancerSettings`, `LoadbalancerSettingsProvider` | kept | neither the contract nor the engine balances; one client per backend behind the endpoint transport |

## Resilience and auth crates

| Crate | Disposition | Reason |
|---|---|---|
| `grpc-egress-retry`, `-retry-std` | kept, over the contract | the contract has none; generic over `Transport + RawTransport`, retries unary only, classifies by `StatusCode`; its typed unary call encodes once and retries over bytes; streams are forwarded (the old client answered `Unimplemented` for them: a defect) |
| `grpc-egress-breaker`, `-breaker-std` | kept, over the contract | same; admits or short-circuits with `Transport(Unavailable)`, records unary outcomes and stream opens; streams forwarded (same defect fixed) |
| `grpc-egress-resilient`, `-resilient-std` | kept, over the contract | retry inside breaker, as ADR-007 |
| `grpc-egress-auth-bearer`, `-auth-bearer-std` | kept | bearer auth is an interceptor; it sets `authorization` through `Metadata`. Whether this crate stays in this repository is for #6, #7 and #8 |

## What this table does not cover

This repository does not know who calls it: dependencies point inward only, so it holds no list of callers and no
plan for their migration. What it publishes for a caller is this table, `behaviour-comparison.md` and the CHANGELOG
(with the example's before and after code as the migration guide).
