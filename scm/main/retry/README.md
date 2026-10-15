# grpc-retry

gRPC-aware retry decorator wrapping any
`GrpcEgress` implementor (the trait of `edge-transport-grpc-egress-transport`).

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
- `config/application.toml` - Default retry policy
