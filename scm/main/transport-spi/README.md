# grpc-egress-transport-spi

Provider-backed gRPC client package. It builds the client through `proto-svc`'s tonic SPI, integrates the load
balancer, and provides cancellation and caller-identity views. It owns the engine and provider dependencies.

This release uses proto-svc v0.4.3 and Tonic 0.14.7. The client SPI retains gzip, deflate, and zstd support;
Tonic's zstd path resolves to zstd 0.14.3 and zstd-safe 8.2.0.

The standard channel, TLS, and interceptor implementations are in the separate
[`grpc-egress-transport-std`](../transport-std/README.md) package. This SPI package uses their public domain factory
contracts to assemble the client; depend on both packages when using the tonic client.

```rust,ignore
use grpc_egress_transport_api::{BuildGrpcEgressBuilderRequest, GrpcEgressBuilderFactory};
use grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory;

let client = TonicGrpcEgressBuilderSpiFactory::new("https://service:443")
    .build(BuildGrpcEgressBuilderRequest)
    .build()?;
```

See the [domain contract](../transport-api/README.md), [workspace overview](../../docs/README.md), and
[ADR-012](../../docs/adr/ADR-012-pattern-egress-lifecycle-adapter.md).
