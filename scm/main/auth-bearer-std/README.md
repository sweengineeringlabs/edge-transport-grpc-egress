# grpc-egress-auth-bearer-std

This package contains standard, dependency-light builders for bearer-auth domain types:
`StdBearerEgressConfigBuilderFactory` and `StdJwtClaimsBuilderFactory`. It has no provider or
cryptography dependencies.

JWT signing and constant-time secret comparison live in the separate
[`grpc-egress-auth-bearer-spi`](../auth-bearer-spi/README.md) package. A composition that needs both
configuration builders and signing depends on both packages.

See the [domain contract](../auth-bearer/README.md).
