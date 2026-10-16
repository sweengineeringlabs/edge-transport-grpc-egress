# grpc-resilient

Resilience decorator for gRPC egress: wraps an already-built `GrpcEgress` client in the retry and circuit-breaker decorators (`GrpcResilientFacade::apply_resilience`). It does not build the client.

## Build

```bash
cargo build
```

## Test

```bash
cargo test
```

## Project Structure

- `src/api/` - Public types and traits (L2)
- `src/core/` - Implementation layer (L3)
- `src/saf/` - Public facade (L4)
- `src/gateway/` - Public entry boundary
- `config/application.toml` - Default resilience policy
