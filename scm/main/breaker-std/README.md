# grpc-egress-breaker-std

**Audience:** Rust developers applying the standard circuit-breaker implementation.

## WHAT

The implementation of the breaker domain `grpc-egress-breaker`: `StdGrpcBreakerFactory::new(config)` wraps
any client that is a `Transport` and a `RawTransport` (the traits of `proto-pattern`) with a circuit breaker. After
`failure_threshold` consecutive failures the circuit opens and calls fail fast, until the cool-down has
passed and probe calls close it again. The built client is also a `BreakerObservable`.

## WHY

The implementation decorates an already-built client and leaves transport construction independent.

## HOW

### Build and test

```bash
cargo build
cargo test
```

### Project structure

- `src/std/` - the breaker client and the types that carry behaviour
- `src/saf/` - the factory

## API

`StdGrpcBreakerFactory` builds the decorator; `BreakerObservable` exposes its state. The inner client must implement Pattern `Transport` and `RawTransport`.

## Documentation

See the [domain contract](../breaker/README.md), [workspace overview](../../docs/README.md), and [ADR-007](../../docs/adr/ADR-007-decorators-wrap-a-built-client.md).
