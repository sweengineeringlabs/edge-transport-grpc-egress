# grpc-egress-auth-bearer

**Audience:** Rust developers configuring bearer authentication for an outbound gRPC client.

## WHAT

The bearer-auth domain of the gRPC egress: the outbound config (`BearerEgressConfig`), the secret
(`BearerSecret`), the claims (`JwtClaims`), the errors and the validation contract. No implementation.

The interceptor that signs a JWT and adds `authorization: Bearer <jwt>` to each outbound request is in
`grpc-egress-auth-bearer-std`.

## WHY

Separating policy/configuration from signing keeps cryptographic implementation dependencies out of the domain API.

## HOW

Use the domain interfaces described above and the standard factory for this crate.
## API

| Contract | Purpose |
|---|---|
| `BearerEgressConfig`, `BearerSecret`, `JwtClaims` | Validated outbound token configuration and claims. |
| `BearerEgressConfigBuilder` and validation contracts | Construct and validate configuration without signing tokens. |

## Documentation

See the [implementation crate](../auth-bearer-std/README.md) for signing and interceptor use, and the [workspace security overview](../../docs/README.md).
