# grpc-egress-transport-std

Implementation of the gRPC egress domain (`grpc-egress-transport`): the client over the `proto-svc`
engine (a private type, built through `TonicGrpcEgressBuilderSpiFactory`), the load-balanced client, the interceptor
chain, the cancellation and security-context views, the `grpc.health.v1` health-check call, the reading of TLS files
(`TlsSettingsResolver`), and the constructors and builders of the domain's data types (`GrpcChannelConfigFactory`,
`MtlsConfigFactory`).

## Usage

```rust,ignore
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;

// The client is a `Transport` and a `RawTransport`.
let client = TonicGrpcEgressBuilderSpiFactory::new("https://service:443")
    .build(BuildGrpcEgressBuilderRequest)
    .build()?;
```

See `examples/` for runnable usage examples.
