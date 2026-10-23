# grpc-egress-auth-bearer-spi

This package contains the provider-backed bearer implementations: the JWT-signing gRPC interceptor and the
constant-time secret guard. It owns the `jsonwebtoken`, `subtle`, and `edge-security` integration dependencies.

The standard, dependency-light builders for `BearerEgressConfig` and `JwtClaims` are in the separate
[`grpc-egress-auth-bearer-std`](../auth-bearer-std/README.md) package. Applications that construct configurations
and use the provider-backed interceptor depend on both packages.

JWT signing supports HS256 with raw secret bytes and RS256 with a PEM private key. The interceptor adds
`authorization: Bearer <jwt>` unless the call already carries an authorization header. Token validation on the
receiving side belongs to the ingress service.

`StdBearerSecretGuardFactory` implements constant-time HS256 secret comparison through `subtle::ConstantTimeEq`.
See the [domain contract](../auth-bearer/README.md), [threat model](docs/threat_model.md), and
[workspace overview](../../docs/README.md).
