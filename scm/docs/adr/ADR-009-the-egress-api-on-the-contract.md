# ADR-009: The egress API on the contract: generic wrappers over `Transport` and `RawTransport`

**Status:** Accepted (the owner, 2026-10-17); implemented on `feat/18-contract-native`; applies the owner's criteria (production grade, zero-cost abstraction, no feature
sacrificed, the `api/std/spi/saf` rules) and ADR-008's rulings 1 to 3
**Date:** 2026-10-17
**Tracking:** [#18](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/18) tasks 5 to 13, 15 to 20
**See also:** [ADR-006](ADR-006-domain-first.md), [ADR-007](ADR-007-decorators-wrap-a-built-client.md), [ADR-008](ADR-008-contract-and-engine-are-external.md)

---

## Context

ADR-008 decided that the contract (`proto-pattern`) and the engine (`proto-svc`) are external, and that a client is a
generic bound, not a `dyn`. The engine swap (task 9, v0.8.x) put the engine behind the old `GrpcEgress` trait with a
bridge. This record fixes what replaces `GrpcEgress` and the types around it, so that every crate is built to one
specification.

## Decision

### 1. One client value, two traits, no `dyn`

The built client is a value that implements both contract traits: [`Transport`] (a call over a `Method`) and
[`RawTransport`] (a call to a `MethodPath`, bytes for messages). It comes from `proto-svc`'s
`TonicTransportSpiFactory` through the new `DualTransportFactory` (`proto-pattern` 0.10.0, `proto-svc` 0.4.0): one
engine, one connection pool. `GrpcEgress`, its `BoxFuture` methods and every `Arc<dyn GrpcEgress>` are removed. Every
consumer is generic: `T: Transport + RawTransport` where it needs both, one of them where it needs one.

### 2. Value types are the contract's

| Removed | Replaced by |
|---|---|
| `GrpcRequest`, `CallStreamRequest` | `MethodPath` + `CallRequest<Vec<u8>>` (+ `CallOptions`: timeout, `Metadata`) |
| `GrpcResponse` | `CallResponse<Vec<u8>, M>` (headers and trailers apart) |
| `GrpcMessageStreamResponse` | `impl ResponseStream<Vec<u8>>` (no boxing) |
| `GrpcEgressError`, `GrpcEgressResult` | `CallError` (a `StatusCode` on every failure) |
| `GrpcStatusCode` | `StatusCode` |
| `HashMap<String, String>` metadata | `Metadata` (many values per key, binary entries) |
| `GrpcRequestFactory`, `GrpcRequestBuilder`, `Conversions`, `CompressionModeProvider` | `CallRequest::create`, `CallOptions::create`, `MetadataBuilder` |
| `GrpcEgressProstCodec`, `ProstUnaryCaller`, the `prost` feature | `Transport::call::<M>` with the caller's own `Payload` |
| `call_stream` (default `Unimplemented`) | removed: every shape has its own method |
| `health_check` (a TCP connect) | the `grpc.health.v1` `Check` call (section 8) |
| `call_unary_with_context`, `CallUnaryWithContextRequest` | the security-context wrapper (section 7) |
| `GrpcEgress` | `Transport` + `RawTransport` |

**Kept, with the reason.** `GrpcChannelConfig`, `MtlsConfig` (PEM *paths*), `KeepAliveConfig` and `CompressionMode`
stay as the **file representation** of a configuration: the contract reads no path and no environment variable, has
no serde, and has no "none" compression, and a configuration file needs all three. They are converted to the
contract's `TlsSettings` and `Compression` (and the engine's keep-alive options) when the client is built, and they
are not a second request, response, error, status, metadata, TLS or compression *type for a call*. The builder
takes the contract's types: `with_tls(TlsSettings)`, `with_send_compression(Compression)`,
`with_accept_compression(Compression)`, `with_max_decoding_message_size`, `with_max_encoding_message_size`.

### 3. Builder

`GrpcEgressBuilder::build(self) -> Result<impl Transport + RawTransport + use<>, GrpcChannelConfigError>`. Options:
`with_endpoint`, `with_plaintext_allowed`, `with_request_timeout`, `with_max_decoding_message_size`,
`with_max_encoding_message_size`, `with_send_compression`, `with_accept_compression` (repeatable), `with_tls`,
`with_keep_alive`, `with_loadbalancer`, `with_interceptor`, `with_trace_context`, `with_channel_config`. A file
configuration is converted by `with_channel_config`: `max_message_bytes` sets **both** message limits (the old
client capped responses only); `compression` sets send and accept; the PEM files of `mtls` are read at `build`, and a
file that cannot be read or holds no PEM item of the kind expected fails with `UnusableTlsFile` naming its path
(`TlsSettingsResolver::resolve` is public so a loader can do the same at load time). A call ends at the earlier of its
own timeout and the request timeout, for every shape (the engine's `with_timeout`).

### 4. Wrappers

A wrapper is a struct holding its inner client that implements both contract traits, so wrappers compose in any
order (ADR-007: interceptors, then retry, then breaker). Each implements the eight methods; a typed method forwards
as a typed call and a raw method as a raw call, so a forwarding wrapper copies nothing. The exception is retry, which
must re-send a request, and a typed message cannot be cloned: its typed unary call encodes the message once with its
`Payload`, sends the original on the first attempt, and rebuilds the message from the encoded bytes for a later
attempt (with one attempt allowed it forwards and encodes nothing). It cannot route a typed call through `call_raw`:
that would need the typed and the raw `Metadata` of the inner client to be one type, which the opaque client the
builder returns does not promise. Every wrapper forwards the three streaming shapes (the old retry and breaker
clients did not forward them and answered `Unimplemented`; that was a defect).

| Wrapper | Where | Behaviour |
|---|---|---|
| Interceptors | `grpc-egress-transport-std` | `before_call` runs in order on every shape and may add metadata or abort with a `CallError` before any request is sent; `after_call` runs in order after a unary or client-streaming response and may fail the call. Built by the builder |
| Load balancer | `grpc-egress-transport-std` | one client per backend, built once; picks by strategy and health, reports the outcome; none healthy is `Transport(Unavailable)` |
| Retry | `grpc-egress-retry-std` | unary only; classifies by `StatusCode` and the trailers (`retry-after`) |
| Breaker | `grpc-egress-breaker-std` | admits or short-circuits with `Transport(Unavailable)`; records the outcome of a unary call and the open of a stream |
| Resilient | `grpc-egress-resilient-std` | retry inside breaker, as ADR-007 |
| Cancellation | `grpc-egress-transport-std` | a view bound to a caller-owned `CancellationSignal` implementation (an interface of the domain): every call, and every message read from a stream, is raced against the signal and fails with `Transport(Cancelled)` |
| Caller identity | `grpc-egress-transport-std` | a view bound to a caller-owned `CallerIdentity` implementation (an interface of the domain): adds `x-trace-id`, `x-tenant-id`, `authorization: Bearer <token>` (when the identity holds a token to send) and the further headers it lists, before the call |
| Bearer auth | `grpc-egress-auth-bearer-std` | an interceptor that signs a JWT into `authorization` |

The interceptor list stays a runtime list (`Arc<dyn GrpcEgressInterceptor>`): it is heterogeneous by nature, and the
cost is one virtual call per interceptor per call, not per message. `GrpcEgress` itself is not `dyn` anywhere.

### 5. The interceptor trait

```rust
pub trait GrpcEgressInterceptor: Send + Sync {
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError>;
    fn after_call(&self, reply: &InboundReply<'_>) -> Result<(), CallError>;
}
```

`OutboundCall` is an **interface** of the domain: the path, the timeout (`get_timeout`, `set_timeout`), the headers
(`has_header`, `get_header_values`) and `add_header`. Its implementation (`PendingCall`) is private to
`grpc-egress-transport-std`, so an interceptor crate, such as the bearer-auth one, depends on the domain crate only.
`InboundReply { path, headers, trailers }` is data: what came back. An interceptor can no longer rewrite the response
*body*: the typed layer has no bytes to rewrite, and no interceptor of this repository did.

### 6. Errors

| Old | Now |
|---|---|
| `Status(code, message)` | `CallError::Status` (the peer's code, message, details, trailers) |
| `ConnectionFailed`, `Unavailable` | `CallError::Transport` with `Unavailable` |
| `Timeout` | `CallError::Status` with `DeadlineExceeded` (the engine reports a passed deadline as a status) |
| `Cancelled` | `CallError::Transport` with `Cancelled` |
| `Internal` | by producer: a request that cannot be encoded is `Encode`, a response that cannot be decoded is `Decode`; an interceptor, breaker or token failure is `Transport` with its own code (`Internal` for a token that cannot be minted) |

A path or a metadata key that is not valid is refused when the `MethodPath` or `MetadataEntry` is created, not at the
call.

### 7. Cancellation and the caller's identity

Dropping a call's future cancels the call (tonic cancels the stream when the future is dropped; a test proves the peer
sees it end). A caller that holds a way to say "stop" keeps it, behind an interface: `CancellationSignal` has one
method, `wait_cancelled`, that resolves when the calls must end; the view races every call and every stream read against
it. A caller implements `CancellationSignal` for its token wrapper.

The identity of the caller is carried the same way, by a view, because the old method took the context per call and the
contract has no per-call extension point. The view knows `CallerIdentity` (a trace id, a tenant, a bearer token that is
to be sent, further headers); a caller-owned type adapts a `SecurityContext`. The identity is given to the factory
as `&dyn CallerIdentity` and read once, when the view is made, so the view does not keep it borrowed.

### 8. Health

The `grpc.health.v1.Health/Check` call as a `Method` (`HealthCheck`) with its messages, in `grpc-egress-transport-api`
(`Payload` by `prost`, a dependency of that crate). The old `health_check` opened a TCP connection and said nothing about the
service; this asks it. Tested against a real health service (`tonic-health`).

## Alternatives and trade-offs

Each decision above was a choice between real alternatives; this is what each cost.

| Decision | Chosen, and what it costs | Rejected, and why |
|---|---|---|
| Hold the client as a generic bound (`T: Transport + RawTransport`), never a `dyn` | No virtual call and no boxed future per call; the compiler specialises each use. **Cost:** code that must choose the client type at run time (a registry of mixed clients) has to be generic itself or wrap the client in its own `dyn`; the compiler generates one copy per client type | An object-safe trait in front of the contract: it is a second interface to keep in step, and every async method of it returns a boxed future, an allocation per call, which the criteria forbid |
| One client value that is both traits (`DualTransportFactory`) | One engine, one connection pool for typed and raw calls; wrappers forward each call as the kind it was | Two clients per endpoint (two pools, two connections to one peer); or the typed call as an encode, raw call and decode on top of the raw one (two extra buffer copies on every typed call) |
| Retry re-sends a typed message by encoding it once and rebuilding it for a later attempt | The first attempt sends the original and copies nothing | Routing a typed call through the raw call: it needs the typed and the raw metadata of the inner client to be one type, which the opaque client does not promise |
| The views (cancellation, identity) know an interface, with an adapter for each library type | A caller keeps the mechanism it has; the domain API names no type of a library; **cost:** one small trait and one adapter each | The library type in the signature (`CancellationToken`, `SecurityContext`): a concrete external type in a port ties every caller to that library |
| The identity is taken as `&dyn CallerIdentity` | One virtual call per header read, once, at build | A type parameter: the precise-capture list must name every type parameter, so the view would be treated as holding the identity and keep it borrowed |
| `OutboundCall` is an interface; interceptors edit the call through it | An interceptor crate depends on the domain only; **cost:** one virtual call per method per interceptor per call | A struct with an extension trait in the implementation crate: the interceptor crate would depend on the implementation |
| The interceptor list stays `Vec<Arc<dyn GrpcEgressInterceptor>>` | One virtual call per interceptor per call, not per message; the builder's `with_interceptor` keeps returning `Self` | A statically composed chain: the builder would change its type at every `with_interceptor` |
| `grpc-egress-resilient-std` names the concrete retry and breaker factories | It is the composition of those two, as ADR-007 says, under one policy | Generic parameters for both factories: the unified `ResilienceConfig` could no longer derive each one's policy |

## Consequences

- Breaking for every caller (`GrpcEgress` is gone). This repository does not know its callers and holds no list of them:
  the CHANGELOG carries the migration guide (the example's before and after code), and each caller migrates on its own.
- Every wrapper has eight small methods; there is no shared abstract "decorator", on purpose: each forwards, and
  the compiler monomorphises it.
- A typed call through a forwarding wrapper copies nothing; through retry it is encoded once, and a later attempt
  decodes the message again, which is what retrying a typed request requires.
