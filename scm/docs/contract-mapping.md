# Contract mapping: every public item against `proto-pattern` and `proto-svc`

Task 1 of `edge-transport-grpc-egress#18`. Checked against this repository at `dev` (v0.8.0), `proto-pattern`
v0.9.1 (`Transport`, `RawTransport`, `MethodPath`) and `proto-svc` v0.3.0 (`TonicTransportSpiFactory` implements both
factories). Each row is **replaced** (by a contract item), **kept** (with the reason) or **removed** (with the reason).
A row that waits for a ruling of tasks 2, 3 or 4 says which.

## The finding that shapes the rest

`GrpcEgress` is a byte-oriented call over a path given as a string: `GrpcRequest { method: String, body: Vec<u8>, .. }`.
That is `RawTransport` (`call_raw(MethodPath, CallRequest<Vec<u8>>)`), not `Transport`, whose path is a constant of a
type. `proto-pattern#19` and `proto-svc#4` exist for this reason. So a consumer that calls by path keeps its call
shape and moves to `RawTransport`; a consumer that has a `Method` type uses `Transport`; both come from one factory
(`TonicTransportSpiFactory`) over one engine.

## Port and call types (`grpc-egress-transport`, `api/client`)

| Item | Disposition | Contract item or reason |
|---|---|---|
| `GrpcEgress` | replaced | `RawTransport` for call-by-path, `Transport` for call-by-`Method`. Not object-safe: **`dyn` use waits for ruling 2** (15 non-test files) |
| `GrpcEgress::call_unary` | replaced | `RawTransport::call_raw` / `Transport::call` |
| `GrpcEgress::call_server_stream`, `call_client_stream`, `call_bidi_stream` | replaced | `call_raw_server_stream`, `call_raw_client_stream`, `call_raw_bidi_stream` / the typed three |
| `GrpcEgress::call_stream` (default `Unimplemented`) | removed | no equivalent; it was the pre-streaming-split entry point and every real shape has its own method. Verify no consumer calls it before the release (task 15) |
| `GrpcEgress::call_unary_with_context` | **waits for ruling 4** | a wrapper that derives metadata from `SecurityContext`, or removal |
| `GrpcEgress::health_check` | **waits** (task 12) | a `grpc.health.v1` `Method`, or removal with a reason |
| `GrpcRequest` | replaced | `CallRequest<Vec<u8>>` + `MethodPath` (+ `CallOptions`). Its `cancellation` field **waits for ruling 3** |
| `GrpcResponse` | replaced | `CallResponse<Vec<u8>, Metadata>` (headers and trailers separate) |
| `GrpcMessageStreamResponse` | replaced | `impl ResponseStream<Vec<u8>>` (no boxing) |
| `CallStreamRequest` | removed | with `call_stream` |
| `CallUnaryWithContextRequest` | **waits for ruling 4** | with `call_unary_with_context` |
| `HealthCheckRequest` | **waits** (task 12) | with `health_check` |
| `GrpcEgressError` (6 string variants) | replaced | `CallError` with a `StatusCode` on every failure; per-variant map is task 6 |
| `GrpcEgressResult` | replaced | `Result<_, CallError>` |
| `GrpcStatusCode` | replaced | `StatusCode` (same 17; adds the HTTP and HTTP/2 mappings) |
| `GrpcEgressBuilder`, `GrpcEgressBuilderFactory`, `BuildGrpcEgressBuilderRequest` (#16) | kept | the builder is this crate's composition seam (interceptors, retry, breaker, load balancing); it builds a `RawTransport`/`Transport` instead of a tonic client. Its traits mirror `TransportFactory`'s shape |
| `GrpcEgressProstCodec` | removed | typed calls are `Transport::call::<M>` with `Payload`; the opt-in prost marker has no remaining job. `proto-svc`'s test peer shows a prost `Payload` |
| `ProstUnaryCaller` (`std/codec`) | removed | same reason; verify no consumer uses it (task 15) |

## Channel configuration (`api/channel`)

| Item | Disposition | Contract item or reason |
|---|---|---|
| `GrpcChannelConfig` | kept | it is the loaded configuration (TOML) this crate owns; it feeds `TonicTransportSpiFactory`'s `with_*` values. The fields it carries are replaced individually, below |
| `GrpcChannelConfigError` | kept | configuration errors are this crate's; not a `CallError` |
| `DEFAULT_MAX_MESSAGE_BYTES`, `DEFAULT_REQUEST_TIMEOUT_SECS` | kept | defaults of this crate's loader; `proto-svc` adds none of its own |
| `MtlsConfig` (file paths) | replaced | `TlsSettings` (PEM bytes). The loader reads the files and builds bytes; an unreadable file fails at load, naming the path (task 7; closes #13) |
| `CompressionMode` | replaced | `Compression` (`None` becomes "unset"; send and accept set separately) (task 8) |
| `KeepAliveConfig` | replaced | `with_http2_keep_alive_interval`, `with_keep_alive_timeout`, `with_keep_alive_while_idle`, `with_tcp_keepalive` (task 8) |
| `request_timeout_secs`, `max_message_bytes` fields | replaced | `with_timeout`; `with_max_decoding_message_size` and `with_max_encoding_message_size` |
| `GrpcChannelConfigBuilder`, `GrpcChannelConfigFactory`, `MtlsConfigFactory`, `CompressionModeProvider` | kept | build this crate's configuration; their values change with the replaced fields |

## Interceptors and trace (`api/interceptor`)

| Item | Disposition | Contract item or reason |
|---|---|---|
| `GrpcEgressInterceptor` (`before_call`, `after_call`) | kept | the contract has no interceptor concept; becomes a wrapper `Transport`/`RawTransport` (task 11). Its request/response types become the contract's |
| `AfterCallRequest` | kept | follows the interceptor; carries the contract's types |
| `GrpcEgressInterceptorFactory`, `BuildGrpcEgressInterceptorRequest` | kept | construction seam of the interceptor |
| `GrpcEgressInterceptorChain`, `EgressInterceptorChain` | kept | ordering is this crate's (ADR-007: interceptors < retry < breaker) |
| `TraceContextGrpcEgressInterceptor`, `TraceContextSource` | kept | W3C trace headers are not in the contract; metadata entries through `Metadata` |

## Load balancing (`api/loadbalancer`)

| Item | Disposition | Contract item or reason |
|---|---|---|
| `BackendTarget`, `BalancingStrategy`, `LoadbalancerSettings`, `LoadbalancerSettingsProvider` | kept | neither the contract nor `proto-svc` balances; a wrapper `Transport` over several `TonicTransportSpiFactory` transports (task 10) |

## Request building (`std/client`)

| Item | Disposition | Contract item or reason |
|---|---|---|
| `GrpcRequestBuilder`, `GrpcRequestFactory` | replaced | `CallRequest::create(message, CallOptions::create(timeout, Metadata))` |
| `Conversions` | removed | converted between this crate's request/response types and tonic's; both ends are the contract's now (verify nothing else uses it) |

## Resilience and auth crates

| Crate | Disposition | Reason |
|---|---|---|
| `grpc-egress-retry` (`GrpcRetryConfig`, `GrpcRetryFactory`, `GrpcRetryClient`, `RetryDecision`, `ResourceExhaustedContext`) | kept | the contract has none. Becomes generic over the contract's transport; classifies by `StatusCode` (task 13). `GrpcRetryClient` was a wrapper of `GrpcEgress`: it wraps `RawTransport`/`Transport` |
| `grpc-egress-breaker` (`GrpcBreakerConfig`, `GrpcBreakerFactory`, `GrpcBreakerClient`, `BreakerObservable`) | kept | same; breaker state classification by `StatusCode` |
| `grpc-egress-resilient` (`ResilienceConfig`, `GrpcResilientFactory`, `ResilientTransportError`) | kept | composition of the two; `ResilientTransportError` is its own configuration error |
| `grpc-egress-auth-bearer` (`BearerEgressConfig`, `BearerSecret`, `Validator`, `BearerAuthError`, `JwtClaimsBuilder`, `StdBearerInterceptorFactory`) | kept | bearer auth is an interceptor; it sets `authorization` through `Metadata`. **Its crate decision stays with #6/#7/#8** |

## What this table does not decide

- Ruling 2 (`dyn`), ruling 3 (cancellation), ruling 4 (security context): rows marked above.
- Whether `call_stream`, `Conversions` and `ProstUnaryCaller` have a consumer: removed here on the evidence of the
  crate itself; task 15 checks every dependent, and a consumer found would turn the row into "kept" with a reason.
