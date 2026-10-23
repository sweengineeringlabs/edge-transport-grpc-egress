# grpc-egress-resilient-std

**Audience:** Rust developers applying the standard resilience composition.

## WHAT

The implementation of the resilience domain `grpc-egress-resilient`: `StdGrpcResilientFactory::new(config)`
wraps an already-built `Transport + RawTransport` client in retry and then a circuit breaker. It does not build the
client.

## WHY

The factory wraps an already-built client, preserving the separate transport and decorator layers.

## HOW

### Build and test

```bash
cargo build
cargo test
```

### Project structure

- `src/std/` - the resilience policy and its conversions
- `src/saf/` - the factory

## API

`StdGrpcResilientFactory` applies the retry and breaker decorators to an existing Pattern `Transport + RawTransport` client. Its order is interceptors, retry, then breaker.

## Documentation

See the [domain contract](../resilient/README.md) and [ADR-007](../../docs/adr/ADR-007-decorators-wrap-a-built-client.md).
