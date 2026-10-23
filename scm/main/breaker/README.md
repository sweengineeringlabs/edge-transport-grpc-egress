# grpc-egress-breaker

**Audience:** Rust developers configuring circuit-breaker behavior for a gRPC client.

## WHAT

The circuit-breaker domain of the gRPC egress: the contracts (`BreakerDecorator`, `BreakerObservable`,
...), their requests and responses, the errors and the breaker policy value (`GrpcBreakerConfig`).
No implementation.

The decorator that wraps a `Transport + RawTransport` client with a circuit breaker is in `grpc-egress-breaker-spi`.

## WHY

A separate policy contract lets the breaker wrap any client implementing Pattern `Transport` and `RawTransport`.

## HOW

Use the domain interfaces described above and the standard factory for this crate.

## API

| Contract | Purpose |
|---|---|
| `BreakerDecorator`, `BreakerObservable` | Decorate and observe a client. |
| `GrpcBreakerConfig`, requests, responses, and errors | Carry validated breaker policy and operations. |

## Documentation

See the [SPI implementation](../breaker-spi/README.md) and [ADR-007](../../docs/adr/ADR-007-decorators-wrap-a-built-client.md).
