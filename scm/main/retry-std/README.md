# grpc-egress-retry-std

**Audience:** Rust developers applying the standard retry implementation.

## WHAT

The implementation of the retry domain `grpc-egress-retry`: `StdGrpcRetryFactory::new(config)` wraps any
client that is a `Transport` and a `RawTransport` (the traits of `proto-pattern`) so that unary calls are retried with exponential
backoff and jitter on `Unavailable` and `ResourceExhausted`, within the caller's deadline.

## WHY

The implementation decorates an already-built client and keeps retry policy outside the transport.

## HOW

### Build and test

```bash
cargo build
cargo test
```

### Project structure

- `src/std/` - the retry client and the types that carry behaviour
- `src/saf/` - the factory

## API

`StdGrpcRetryFactory` builds a retry decorator around an existing Pattern `Transport + RawTransport` client. Streaming calls are forwarded without replay.

## Documentation

See the [domain contract](../retry/README.md), [workspace overview](../../docs/README.md), and [ADR-007](../../docs/adr/ADR-007-decorators-wrap-a-built-client.md).
