# grpc-egress-retry-std

The implementation of the retry domain `grpc-egress-retry`: `StdGrpcRetryFactory::new(config)` wraps any
`GrpcEgress` (the trait of `grpc-egress-transport`) so that unary calls are retried with exponential
backoff and jitter on `Unavailable` and `ResourceExhausted`, within the caller's deadline.

## Build and test

```bash
cargo build
cargo test
```

## Project structure

- `src/std/` - the retry client and the types that carry behaviour
- `src/saf/` - the factory
