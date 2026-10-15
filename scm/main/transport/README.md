# edge-transport-grpc-egress-transport

gRPC outbound transport crate for the swe-edge egress layer.

Provides `GrpcEgress` — the outbound port trait for gRPC calls — and the
a concrete client backed by `tonic` over HTTP/2 (a private type, built through `TransportConstruction`).

## Usage

```rust
use edge_transport_grpc_egress_transport::{GrpcChannelConfig, TransportConstruction};

let cfg = GrpcChannelConfig::new("https://service:443");
let client = TransportConstruction::create_transport_from_config(&cfg)?;
```

See `examples/` for runnable usage examples.
