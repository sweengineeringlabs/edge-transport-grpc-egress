# grpc-egress-transport

**Audience:** Rust developers building and configuring outbound gRPC clients.

## WHAT

The gRPC egress domain: the builder of a client and the options it takes, the interceptor that runs around a call,
and the factories of the views a client is wrapped in. The client is a `proto-pattern` `Transport` and `RawTransport`:
its calls, requests, responses, errors, status codes and metadata are the contract's. Plain data and traits; no
implementation and no heavy dependency (no `tonic`, `hyper`, `rustls`, `tower`, load balancer).

The implementation is `grpc-egress-transport-std`: a client built through
`TonicGrpcEgressBuilderSpiFactory`, with the builders and constructors for these types.

Themes: `client`, `channel`, `interceptor`, `loadbalancer`, `cancellation` and `context`.

## WHY

The domain contract can be used without taking a dependency on tonic or its TLS implementation.

## HOW

Use the domain interfaces described above and the standard factory for this crate.

## API

| Contract | Purpose |
|---|---|
| `GrpcEgressBuilder`, `GrpcEgressBuilderFactory` | Configure and build a typed and raw transport client. |
| `GrpcChannelConfig`, TLS and load-balancer value objects | Carry validated client settings. |
| `GrpcEgressInterceptor` and view factories | Observe and modify calls, and expose cancellation and caller identity. |

## Documentation

See the [standard implementation](../transport-std/README.md), [workspace overview](../../docs/README.md), and [ADR-012](../../docs/adr/ADR-012-pattern-egress-lifecycle-adapter.md).
