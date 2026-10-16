# ADR-008: The contract and the engine are external (`proto-pattern`, `proto-svc`)

**Status:** Proposed, not implemented. Issue 18 task 2, 3 and 4 are the owner's to rule on; this record states the facts found while preparing the mapping (task 1), a recommendation for each ruling, and one finding that the issue does not name and that decides how much of the move is possible.
**Date:** 2026-10-16
**Tracking:** [#18](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/18) tasks 1 to 4
**See also:** [ADR-006](ADR-006-domain-first.md), [ADR-007](ADR-007-decorators-wrap-a-built-client.md)

---

## Context

`proto-pattern` (v0.8.2) is the contract of one typed call over a `Method`, with unary and the three streaming shapes. `proto-svc` (v0.2.0) is its `tonic` implementation. This repository is the same kind of thing with its own contract (`GrpcEgress`, raw bytes). Issue 18 asks that the transport concern live in one place and the contract in another.

## Finding that the issue does not name: the path is a constant of the type

`Method::PATH` is an associated constant: `transport.call::<Greet>(request)`. `GrpcRequest::method` is a **string chosen at run time**: every call site in this repository and its tests passes one. Over the contract such a call has no expression without one `Method` type per path, which code that holds only a string cannot create. Whether a real consumer needs the path dynamic (a router, a generic client) is to be confirmed in task 15; `edge-bootstrap` builds `GrpcRequest`s from a `method` variable in its ingress adapters, but those may be the ingress crates' types. So the byte-oriented, dynamic-path surface of `GrpcEgress` is **not** a wrapper over `Transport`; it is a different capability.

This changes the three rulings below from a choice among wrappers into a choice about what survives.

## Mapping (task 1), final for the public items of today's crates

| Item (crate) | Disposition | Reason |
|---|---|---|
| `GrpcRequest`, `GrpcResponse` (transport) | kept for the dynamic-path surface; typed callers use `CallRequest`/`CallResponse` | the path is a run-time string (above) |
| `GrpcStatusCode` | replaced by `StatusCode` | the same 17 codes; the contract adds the HTTP and HTTP/2 mappings |
| `GrpcEgressError`, `GrpcEgressResult` | replaced by `CallError` for typed calls; kept for the dynamic surface | `CallError` tells encode, decode, transport and status apart; six string variants cannot |
| `GrpcChannelConfig`, `GrpcChannelConfigError` | kept (the builder's input and refusals), fields mapped to proto-svc options | the contract has no loader for TOML and no option validation message that names options |
| `MtlsConfig` | replaced by `TlsSettings` | PEM bytes in memory; the file reading moves to the loader of the config |
| `CompressionMode` | replaced by `Compression` plus separate send and accept options | no `None` in the contract |
| `KeepAliveConfig` | kept as the value of `with_keep_alive`; split into the engine's four options | |
| `GrpcEgress`, `GrpcEgressBuilder`, `GrpcEgressBuilderFactory`, `BuildGrpcEgressBuilderRequest` | kept for the dynamic-path surface and its construction (`TransportFactory` shape) | see the finding |
| `CallStreamRequest`, `GrpcMessageStreamResponse` | replaced by `ResponseStream`/`MessageStream` for typed streams; kept for the dynamic surface | |
| `CallUnaryWithContextRequest`, `SecurityContext` | kept; see ruling 3 | the contract has no security context |
| `HealthCheckRequest` | replaced by a `grpc.health.v1` `Method` defined here, or removed with its reason (task 12) | the contract has no health check |
| `AfterCallRequest`, `GrpcEgressInterceptor`, `GrpcEgressInterceptorFactory`, `BuildGrpcEgressInterceptorRequest`, `TraceContextSource`, `GrpcEgressInterceptorChain`, `TraceContextGrpcEgressInterceptor`, `StdGrpcEgressInterceptorFactory` | kept; they wrap any transport (task 11) | the contract and the engine have neither interceptors nor trace context |
| `BackendTarget`, `BalancingStrategy`, `LoadbalancerSettings`, `LoadbalancerSettingsProvider` | kept (task 10) | no load balancing in the engine |
| `GrpcRequestFactory`, `GrpcRequestBuilder`, `GrpcChannelConfigFactory`, `GrpcChannelConfigBuilder`, `MtlsConfigFactory`, `CompressionModeProvider`, `Conversions` | follow the type each serves | |
| `GrpcEgressProstCodec`, `ProstUnaryCaller` | removed when `Payload` replaces the prost helper: a typed call is `transport.call::<M>` | the contract is the typed layer |
| `TonicGrpcEgressBuilderSpiFactory` (the hand-written `hyper` client) | replaced by `proto-svc`'s `TonicTransportSpiFactory` behind the same builder | task 9 |
| retry, breaker, resilient, auth-bearer factories | kept; they wrap `GrpcEgress` today and take a `Transport` once it is generic (task 13) | |

## Rulings and recommendations

**Ruling 1, the `dyn` question (task 2).** `Transport` is not object-safe. Options: (a) consumers use generic bounds; (b) this crate keeps one object-safe trait in front of the contract.
*Recommendation:* (b) for the dynamic-path surface, which is object-safe by nature (a string path and bytes) and is what `edge-bootstrap` holds as `Arc<dyn GrpcEgress>` (its runtime builder, registry and port crates; this repository has `dyn GrpcEgress` in 15 non-test source files, per the issue); (a) for every typed use. The finding above is the reason: the dynamic surface cannot be a generic over `Method`.

**Ruling 2, cancellation (task 3).** The contract has no token. Options: dropping the call's future cancels it (documented and tested), or a wrapper keeps `CancellationToken`.
*Recommendation:* the wrapper keeps the token on the dynamic surface (it already races the call against it, and tests prove `Cancelled`), and typed callers rely on dropping the future. Removing the token from `GrpcRequest` would remove a capability the owner's rule forbids degrading.

**Ruling 3, security context (task 4).** `call_unary_with_context` has no equivalent. Options: an interceptor that derives metadata from the context and wraps `Transport`, or removal.
*Recommendation:* the interceptor, so the capability stays and the engine stays unaware of it.

## What is not decided here

Whether the dynamic-path surface should itself move into `proto-pattern` (a `Method` with a run-time path) is a question for the owner of that crate; this record only shows that without it the contract cannot replace `GrpcEgress`. No code changes until the rulings are given.
