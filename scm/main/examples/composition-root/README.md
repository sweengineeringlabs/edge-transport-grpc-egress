# grpc-egress-example-composition-root

**Audience:** Developers integrating the egress crates and maintainers verifying the public API.

## WHAT

What a composition root does with the gRPC egress crates, using nothing but their public API. It loads a
`GrpcChannelConfig` from TOML text, assembles a client from it and from options, and wraps the client in
the retry and breaker decorators. The client is a `proto-pattern` `Transport` and `RawTransport`; the root holds it
through a generic bound, never a `dyn`. An in-process peer stands in for the server, so no network is needed.

## WHY

A consumer-shaped example proves that clients can be assembled and decorated using only public APIs, with observable behavior against real in-process gRPC peers.

## HOW

```bash
cargo run -p grpc-egress-example-composition-root                    # one line per scenario, exit 0
cargo test -p grpc-egress-example-composition-root
```

### The consumer code

This is the whole of what a root writes to start from a configuration file (`CompositionRoot::assemble`) and to
make a call through any client (`CompositionRoot::ping`); the options are added to the builder `assemble` returns,
and `build()` ends it:

```rust,ignore
/// Start a builder with the loaded configuration applied; the caller adds options and builds.
///
/// # Errors
///
/// The error of loading the configuration.
pub fn assemble(&self) -> Result<impl GrpcEgressBuilder, Box<dyn Error>> {
    let config = self.load_config()?;
    let builder = TonicGrpcEgressBuilderSpiFactory::new(config.endpoint().to_owned())
        .build(BuildGrpcEgressBuilderRequest)
        .with_channel_config(&config);
    Ok(builder)
}

/// A call made through any client, bare or wrapped: generic over the contract's trait, never a `dyn`.
///
/// # Errors
///
/// The error of the path or of the call.
pub async fn ping<T: RawTransport>(client: &T, body: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let path = MethodPath::create("/pkg.Service/Method")?;
    let request = CallRawRequestBuilder::new()
        .with_path(path)
        .with_message(body.to_vec())
        .with_options(CallOptions::default())
        .build();
    Ok(client
        .call_raw(request)
        .await?
        .into_response()
        .into_message())
}
```

A test fails when this quote differs from `src/composition_root.rs`.

### The nine scenarios

The current scenarios cover transport options, configuration, interceptors and lifecycle events, balancing, decorators, invalid combinations, generic clients, cancellation, and caller identity. The removed prost-codec scenario is not part of the current contract; typed calls use `proto-pattern::Payload`. Each scenario runs against in-process peers:

1. every transport option (`AllOptionsScenario`)
2. a loaded config and one overriding option (`LoadedConfigScenario`)
3. interceptors and the trace context (`InterceptorScenario`), including Pattern `CallEvent` lifecycle observations through the existing runtime chain
4. a load balancer over several backends (`LoadbalancerScenario`)
5. retry and breaker applied by the root after `build()` (`DecoratorScenario`)
6. every invalid combination (`InvalidCombinationScenario`)
7. ruling 1, a client is a generic bound: the bare, retried and guarded client go through one generic function, and a
   typed call goes through the same stack (`GenericClientScenario`)
8. ruling 2, dropping a call's future cancels it, and a token can too: against a real `tonic` server (`CancellationScenario`)
9. ruling 3, the caller's identity is headers of the call (`SecurityContextScenario`)

### What a composition root does, and what the builder changes

A composition root that built clients with the removed factory functions
(`create_*_from_config`) and wrapped them itself makes, with the builder, one call chain per client:
the factory, the options, `build()`. Retry and breaker stay outside the transport (ADR-004): the root
wraps the built client with each decorator crate's own factory, as scenario 5 does.

## API

`CompositionRoot` assembles a builder from loaded config; `ping` is generic over `RawTransport`. Each scenario is a public-API consumer and its effects are asserted in `tests/composition_root_int_test.rs`.

## Documentation

See the [workspace overview](../../../docs/README.md), [ADR-007](../../../docs/adr/ADR-007-decorators-wrap-a-built-client.md), and [ADR-012](../../../docs/adr/ADR-012-pattern-egress-lifecycle-adapter.md).
