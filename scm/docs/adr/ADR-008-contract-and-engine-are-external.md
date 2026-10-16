# ADR-008: The contract and the engine are external (`proto-pattern`, `proto-svc`)

**Status:** Proposed, not implemented. The owner's criteria (production grade, zero-cost abstraction, no feature sacrificed, the `api/std/spi/saf` rules) are applied to rulings 1 to 3 below; acceptance of the rulings is requested before the code moves. The finding that the path is a constant of the type is resolved: `proto-pattern` 0.9 and `proto-svc` 0.3 provide `RawTransport`.
**Date:** 2026-10-16 (revised after `proto-pattern` 0.9.1 and `proto-svc` 0.3.0)
**Tracking:** [#18](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/18) tasks 1 to 4
**See also:** [ADR-006](ADR-006-domain-first.md), [ADR-007](ADR-007-decorators-wrap-a-built-client.md)

---

## Context

`proto-pattern` (v0.8.2) is the contract of one typed call over a `Method`, with unary and the three streaming shapes. `proto-svc` (v0.2.0) is its `tonic` implementation. This repository is the same kind of thing with its own contract (`GrpcEgress`, raw bytes). Issue 18 asks that the transport concern live in one place and the contract in another.

## Finding that the issue did not name: the path is a constant of the type (resolved)

`Method::PATH` is an associated constant: `transport.call::<Greet>(request)`. `GrpcRequest::method` is a **string chosen at run time**. Over the contract of 0.8 such a call had no expression, so the byte-oriented, dynamic-path surface of `GrpcEgress` was not a wrapper over `Transport`. This was raised as [proto-pattern#19](https://github.com/sweengineeringlabs/proto-pattern/issues/19), which added `RawTransport` (a `MethodPath` value and byte messages, the four call shapes) in 0.9.0 and `RawTransportFactory` in 0.9.1 (ADR-008 there), and `proto-svc#4`, which implements both over one engine in 0.3.0. So the dynamic surface **is** a wrapper over a contract trait: `RawTransport`. The rulings below follow from that.

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

## Rulings (applying the owner's criteria)

The criteria: production grade; zero-cost abstraction; no feature sacrificed; the `api/`, `std/`, `spi/` and `saf/` rules.

**Ruling 1, the `dyn` question (task 2): generic bounds, no `dyn`.** Every consumer is generic over `Transport` or `RawTransport` (or holds the concrete type a factory returned). A `dyn GrpcEgress` costs a vtable call and, today, a boxed future per call (`BoxFuture`); a generic bound costs neither. Nothing is lost: `RawTransport` has the dynamic path, so the one capability an object-safe trait protected (call by a run-time string) is a generic method now. An object-safe wrapper would add back the cost the criteria forbid and is not kept. The consequence is that `edge-bootstrap`'s `RuntimeBuilder` and the registries that hold `Arc<dyn GrpcEgress>` become generic over the transport; task 15 lists the change per consumer. *This reverses the earlier recommendation (b), which existed only because `RawTransport` did not.*

**Ruling 2, cancellation (task 3): dropping the call's future cancels it, and the token stays as a wrapper.** The engine's calls are futures; dropping one cancels the call (to be documented and proved against a real server). The `CancellationToken` of `GrpcRequest` is a capability and is kept: a wrapper over `RawTransport`/`Transport` races the call against the token and reports `Cancelled`, and a transport used without it pays nothing (the wrapper is a type the caller chooses to build; no field is added to the contract). Removing the token would remove a feature.

**Ruling 3, security context (task 4): an interceptor wrapper.** An interceptor derives metadata from the `SecurityContext` and wraps the transport, so the capability stays, the engine and the contract stay unaware of it, and a transport used without it pays nothing.

## Layering

The contract types are named in `api/` through the ports of this crate (the contract is a dependency, not copied). Wrappers (interceptor, retry, breaker, load balancer, cancellation, security context) are `std/` types built by one `saf/` factory per theme, as in ADR-006 and ADR-007. The engine (`proto-svc`) is reached only from the `-std` crate's `spi/`.

## How the obstacle was removed

`proto-pattern#19` (`RawTransport`, `MethodPath`, `RawTransportFactory`; released 0.9.0 and 0.9.1) and `proto-svc#4` (implementation; 0.3.0).

## What is not decided here

Whether the bearer-auth crate stays (#6, #7, #8). Whether any consumer depends on `call_stream`, `Conversions` or `ProstUnaryCaller` (task 15). The moves are staged: tasks 5 to 8 (value types), then 9 (the engine), then the wrappers.
