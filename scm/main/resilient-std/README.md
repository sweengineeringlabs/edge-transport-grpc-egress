# grpc-egress-resilient-std

The implementation of the resilience domain `grpc-egress-resilient`: `StdGrpcResilientFactory::new(config)`
wraps an already-built `GrpcEgress` client in retry and then a circuit breaker. It does not build the
client.

## Build and test

```bash
cargo build
cargo test
```

## Project structure

- `src/std/` - the resilience policy and its conversions
- `src/saf/` - the factory
- `config/application.toml` - default resilience policy
