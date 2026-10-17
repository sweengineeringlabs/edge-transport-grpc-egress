# grpc-egress-example-composition-root

What a composition root does with the gRPC egress crates, using nothing but their public API. It loads a
`GrpcChannelConfig` from TOML text, assembles a client from it and from options, and wraps the client in
the retry and breaker decorators. The client is a `proto-pattern` `Transport` and `RawTransport`; the root holds it
through a generic bound, never a `dyn`. An in-process peer stands in for the server, so no network is needed.

```bash
cargo run -p grpc-egress-example-composition-root                    # one line per scenario, exit 0
cargo test -p grpc-egress-example-composition-root
```

## The consumer code

This is the whole of what a root writes to start from a configuration file (`CompositionRoot::assemble`) and to
make a call through any client (`CompositionRoot::ping`); the options are added to the builder `assemble` returns,
and `build()` ends it:

```rust
/// Start a builder with the loaded configuration applied; the caller adds options and builds.
///
/// # Errors
///
/// The error of loading the configuration.
pub fn assemble(&self) -> Result<impl GrpcEgressBuilder, Box<dyn Error>> {
    let config = self.load_config()?;
    let builder = TonicGrpcEgressBuilderSpiFactory::new(config.endpoint.clone())
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
    let request = CallRequest::create(body.to_vec(), CallOptions::default());
    Ok(client.call_raw(path, request).await?.into_message())
}
```

A test fails when this quote differs from `src/composition_root.rs`.

## The nine scenarios

The first six are the scenarios of the restructuring (a seventh, the prost codec option, went with the option: the
engine always sends `application/grpc`). The last three show a ruling of ADR-008 each, as running code, against a
real server:

1. every transport option (`AllOptionsScenario`)
2. a loaded config and one overriding option (`LoadedConfigScenario`)
3. interceptors and the trace context (`InterceptorScenario`)
4. a load balancer over several backends (`LoadbalancerScenario`)
5. retry and breaker applied by the root after `build()` (`DecoratorScenario`)
6. every invalid combination (`InvalidCombinationScenario`)
7. ruling 1, a client is a generic bound: the bare, retried and guarded client go through one generic function, and a
   typed call goes through the same stack (`GenericClientScenario`)
8. ruling 2, dropping a call's future cancels it, and a token can too: against a real `tonic` server (`CancellationScenario`)
9. ruling 3, the security context is headers of the call (`SecurityContextScenario`)

## What a composition root does, and what the builder changes

A composition root that built clients with the removed factory functions
(`create_*_from_config`) and wrapped them itself makes, with the builder, one call chain per client:
the factory, the options, `build()`. Retry and breaker stay outside the transport (ADR-004): the root
wraps the built client with each decorator crate's own factory, as scenario 5 does.
