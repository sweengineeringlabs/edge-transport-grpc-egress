# grpc-egress-transport-std

Standard, dependency-light implementations of the transport domain's channel configuration, TLS resolution, and
interceptor contracts. This package contains no provider-engine or third-party dependencies.

The provider-backed client over `proto-svc`, along with cancellation, caller-identity, and load-balancer support, is
in [`grpc-egress-transport-spi`](../transport-spi/README.md). Applications that build the provider-backed client
depend on both packages.

See the [domain contract](../transport/README.md).
