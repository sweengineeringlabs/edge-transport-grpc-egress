# grpc-egress-transport-std

**Audience:** Rust developers constructing outbound gRPC clients with the standard implementation.

## WHAT

Implementation of the gRPC egress domain (`grpc-egress-transport`): the client over the `proto-svc`
engine (a private type, built through `TonicGrpcEgressBuilderSpiFactory`), the load-balanced client, the interceptor
chain, the cancellation and security-context views, the `grpc.health.v1` health-check call, the reading of TLS files,
and the builders of the domain's data types. Everything a caller uses is an interface of `grpc-egress-transport`
(`TlsSettingsResolver`, `GrpcChannelConfigBuilder`, `MtlsConfigFactory`, ...); this crate implements them on private
types, and a `Std*Factory` of its `saf/` creates each one. `std/` holds the defaults and names no third-party crate;
`spi/` holds what does (`tokio`, `tokio-util`, `prost`, `toml`, `proto-svc`).

## WHY

The crate supplies the production factories while keeping implementation types behind the transport domain interfaces.

## HOW

### Usage

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

## API

`TonicGrpcEgressBuilderSpiFactory` creates the public domain builder. `Std*Factory` types create the channel-config, TLS, interceptor, cancellation, and caller-identity implementations. The returned client implements Pattern `Transport` and `RawTransport`.

## Documentation

See the [domain contract](../transport/README.md), [workspace overview](../../docs/README.md), [architecture](../../docs/architecture.md), and [ADR-012](../../docs/adr/ADR-012-pattern-egress-lifecycle-adapter.md).
