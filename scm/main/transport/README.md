# grpc-egress-transport

**Audience:** Rust developers building and configuring outbound gRPC clients.

## WHAT

The gRPC egress domain: the builder of a client and the options it takes, the interceptor that runs around a call,
and the factories of the views a client is wrapped in. The client is a `proto-pattern` `Transport` and `RawTransport`:
its calls, requests, responses, errors, status codes and metadata are the contract's. Plain data and traits; no
implementation and no heavy dependency (no `tonic`, `hyper`, `rustls`, `tower`, load balancer).

Standard channel, TLS, and interceptor implementations are in [`grpc-egress-transport-std`](../transport-std/README.md).
The `proto-svc` client, cancellation, caller-identity, and load-balancer providers are in
[`grpc-egress-transport-spi`](../transport-spi/README.md).

Themes: `client`, `channel`, `interceptor`, `loadbalancer`, `cancellation` and `context`.

## WHY

The domain contract can be used without taking a dependency on tonic or its TLS implementation.

## HOW

Use the domain interfaces described above and depend on the standard and provider packages required by the application.

## API

| Contract | Purpose |
|---|---|
| `GrpcEgressBuilder`, `GrpcEgressBuilderFactory` | Configure and build a typed and raw transport client. |
| `GrpcChannelConfig`, TLS and load-balancer value objects | Carry validated client settings. |
| `GrpcEgressInterceptor` and view factories | Observe and modify calls, and expose cancellation and caller identity. |

## Documentation

See the [standard package](../transport-std/README.md), [SPI package](../transport-spi/README.md), [workspace overview](../../docs/README.md), and [ADR-012](../../docs/adr/ADR-012-pattern-egress-lifecycle-adapter.md).
