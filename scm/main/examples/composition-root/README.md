# grpc-egress-example-composition-root

What a composition root does with the gRPC egress crates, using nothing but their public API. It loads a
`GrpcChannelConfig` from TOML text, assembles a client from it and from options, and wraps the client in
the retry and breaker decorators. An in-process peer stands in for the server, so no network is needed.

```bash
cargo run -p grpc-egress-example-composition-root                    # one line per scenario, exit 0
cargo test -p grpc-egress-example-composition-root
```

## The consumer code

This is the whole of what a root writes to start from a configuration file (`CompositionRoot::assemble`);
the options are added to the builder it returns, and `build()` ends it:

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
```

A test fails when this quote differs from `src/composition_root.rs`.

## The six scenarios (the seventh, the prost codec option, went with the option)

1. every transport option (`AllOptionsScenario`)
2. a loaded config and one overriding option (`LoadedConfigScenario`)
3. interceptors and the trace context (`InterceptorScenario`)
4. a load balancer over several backends (`LoadbalancerScenario`)
5. retry and breaker applied by the root after `build()` (`DecoratorScenario`)
6. every invalid combination (`InvalidCombinationScenario`)

## What `edge-bootstrap` does today, and what the builder changes

`edge-bootstrap`'s composition root builds clients with the removed factory functions
(`create_*_from_config`) and wraps them itself. With the builder it makes one call chain per client:
the factory, the options, `build()`. Retry and breaker stay outside the transport (ADR-004): the root
wraps the built client with each decorator crate's own factory, as scenario 5 does.
