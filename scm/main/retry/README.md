# grpc-egress-retry

**Audience:** Rust developers configuring retries for a gRPC client.

## WHAT

The retry domain of the gRPC egress: the contracts (`BackoffScheduler`, `RetryDecorator`,
`RetryInspector`, ...), their requests and responses, the errors and the retry policy value
(`GrpcRetryConfig`). No implementation.

The standard retry classifier is in [`grpc-egress-retry-std`](../retry-std/README.md). The provider-backed decorator
and TOML policy readers are in [`grpc-egress-retry-spi`](../retry-spi/README.md).

## WHY

A separate retry policy can wrap any client implementing Pattern `Transport` and `RawTransport`.

## HOW

Use the domain interfaces described above and the standard factory for this crate.

## API

| Contract | Purpose |
|---|---|
| `GrpcRetryConfig` | Validated retry policy. |
| `RetryDecorator`, `RetryInspector`, `BackoffScheduler` | Decorate calls, observe decisions, and schedule backoff. |

## Documentation

See the [standard package](../retry-std/README.md), [SPI package](../retry-spi/README.md), and [ADR-007](../../docs/adr/ADR-007-decorators-wrap-a-built-client.md).
