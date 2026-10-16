# grpc-egress-transport-std

Implementation of the gRPC egress domain (`grpc-egress-transport`): the client backed by `hyper`
over HTTP/2 (a private type, built through `TonicGrpcEgressBuilderSpiFactory` or
`TransportConstruction`), the load-balanced client, interceptors, and the constructors and builders
of the domain's data types (`GrpcRequestFactory`, `GrpcChannelConfigFactory`, `MtlsConfigFactory`).

## Usage

```rust
use grpc_egress_transport::GrpcChannelConfig;
use grpc_egress_transport_std::{GrpcChannelConfigFactory, TransportConstruction};

let cfg = GrpcChannelConfig::new("https://service:443");
let client = TransportConstruction::create_transport_from_config(&cfg)?;
```

See `examples/` for runnable usage examples.
