# edge-transport-grpc-egress-transport

gRPC outbound transport crate for the swe-edge egress layer.

Provides `GrpcEgress` — the outbound port trait for gRPC calls — and the
`TonicGrpcClient` concrete implementation backed by hyper HTTP/2.

## Usage

```rust
use edge_transport_grpc_egress_transport::{create_transport_from_config, GrpcChannelConfig};

let cfg = GrpcChannelConfig::new("https://service:443");
let client = create_transport_from_config(&cfg)?;
```

See `examples/` for runnable usage examples.
