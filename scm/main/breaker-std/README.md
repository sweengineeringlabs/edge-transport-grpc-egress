# grpc-egress-breaker-std

The implementation of the breaker domain `grpc-egress-breaker`: `StdGrpcBreakerFactory::new(config)` wraps
any `GrpcEgress` (the trait of `grpc-egress-transport`) with a circuit breaker. After
`failure_threshold` consecutive failures the circuit opens and calls fail fast, until the cool-down has
passed and probe calls close it again. The built client is also a `BreakerObservable`.

## Build and test

```bash
cargo build
cargo test
```

## Project structure

- `src/std/` - the breaker client and the types that carry behaviour
- `src/saf/` - the factory
