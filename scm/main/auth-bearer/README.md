# grpc-egress-auth-bearer

**Audience:** Rust developers configuring bearer authentication for an outbound gRPC client.

## WHAT

The bearer-auth domain of the gRPC egress: the outbound config (`BearerEgressConfig`), the secret
(`BearerSecret`), the claims (`JwtClaims`), the errors and the validation contract. No implementation.

Standard configuration and claims builders are in [`grpc-egress-auth-bearer-std`](../auth-bearer-std/README.md).
The provider-backed JWT interceptor and constant-time secret guard are in
[`grpc-egress-auth-bearer-spi`](../auth-bearer-spi/README.md).

## WHY

Separating policy/configuration from signing keeps cryptographic implementation dependencies out of the domain API.

## HOW

Use the domain interfaces described above and depend on the implementation packages required by the application.
## API

| Contract | Purpose |
|---|---|
| `BearerEgressConfig`, `BearerSecret`, `JwtClaims` | Validated outbound token configuration and claims. |
| `BearerEgressConfigBuilder` and validation contracts | Construct and validate configuration without signing tokens. |

## Documentation

See the [standard package](../auth-bearer-std/README.md), [SPI package](../auth-bearer-spi/README.md), and [workspace security overview](../../docs/README.md).
