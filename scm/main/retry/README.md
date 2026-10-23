# grpc-egress-retry

**Audience:** Rust developers configuring retries for a gRPC client.

## WHAT

The retry domain of the gRPC egress: the contracts (`BackoffScheduler`, `RetryDecorator`,
`RetryInspector`, ...), their requests and responses, the errors and the retry policy value
(`GrpcRetryConfig`). No implementation.

The decorator that wraps a `Transport + RawTransport` client with retry is in `grpc-egress-retry-std`.

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

See the [standard implementation](../retry-std/README.md) and [ADR-007](../../docs/adr/ADR-007-decorators-wrap-a-built-client.md).
