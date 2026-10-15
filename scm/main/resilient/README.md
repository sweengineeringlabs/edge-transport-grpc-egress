# grpc-resilient

Assembled resilient gRPC transport: `TonicGrpcClient` wrapped with retry and circuit-breaker layers.

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
