# grpc-egress-auth-bearer-std

**Audience:** Rust developers adding JWT bearer authentication to an outbound gRPC client.

## WHAT

JWT bearer interceptor for the gRPC egress stack.

The interceptor implements `grpc_egress_transport::GrpcEgressInterceptor`: it
signs a JWT from the configured claims and adds `authorization: Bearer <jwt>` to every outbound
request. Validating the token on the receiving side is not part of this crate; that is the ingress
side's job (`edge-transport-grpc-ingress-auth-bearer`).

Algorithms: HS256 (raw secret bytes) and RS256 (PEM private key).

## WHY

Keeping signing in the implementation crate lets consumers depend on the small bearer domain contract.

## HOW

### Quick start

```rust,ignore
use grpc_egress_auth_bearer::{
    BearerEgressConfigBuilder, BearerEgressConfigBuilderFactory, BearerSecret,
    BuildBearerEgressConfigBuilderRequest,
};
use grpc_egress_auth_bearer_std::{StdBearerEgressConfigBuilderFactory, StdBearerInterceptorFactory};

let config = StdBearerEgressConfigBuilderFactory
    .build(BuildBearerEgressConfigBuilderRequest)
    .secret(BearerSecret::hs256(secret_bytes ).unwrap())
    .issuer("svc-a")
    .audience("svc-b")
    .subject("svc-a-client")
    .lifetime_seconds(60)
    .build()?;
let interceptor = StdBearerInterceptorFactory::new(config).build(BuildGrpcEgressInterceptorRequest);
```

The interceptor is attached to a client with `GrpcEgressBuilder::with_interceptor`.

### Wire shape

| Header | Format |
|--------|--------|
| `authorization` | `Bearer <jwt>` |

### Constant-time secret comparison

`BearerSecretGuard::ct_eq_hs256` (created by `StdBearerSecretGuardFactory`) uses `subtle::ConstantTimeEq` for raw secret material. Token content
comparisons are delegated to `jsonwebtoken`, which verifies the MAC in constant time.

## API

`StdBearerEgressConfigBuilderFactory` constructs the validated config; `StdBearerInterceptorFactory` creates the interceptor attached with `GrpcEgressBuilder::with_interceptor`. `StdBearerSecretGuardFactory` provides constant-time HS256 secret comparison.

## Documentation

See the [domain contract](../auth-bearer/README.md), [threat model](docs/threat_model.md), and [workspace overview](../../docs/README.md).
