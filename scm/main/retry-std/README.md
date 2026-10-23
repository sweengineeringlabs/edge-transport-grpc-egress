# grpc-egress-retry-std

Standard retry policy implementation. `StdRetryClassifierFactory` provides the default gRPC retry classification
through the domain crate's `RetryClassifierFactory` contract. This package contains no provider or transport-engine
dependencies.

The retry decorator and TOML configuration providers are in the separate
[`grpc-egress-retry-spi`](../retry-spi/README.md) package.
