# edge-transport-grpc-egress-auth-bearer

JWT bearer interceptor for the gRPC egress stack.

`BearerEgressInterceptor` implements `grpc_egress_transport::GrpcEgressInterceptor`: it
signs a JWT from the configured claims and adds `authorization: Bearer <jwt>` to every outbound
request. Validating the token on the receiving side is not part of this crate; that is the ingress
side's job (`edge-transport-grpc-ingress-auth-bearer`).

Algorithms: HS256 (raw secret bytes) and RS256 (PEM private key).

## Quick start

```rust,ignore
use edge_transport_grpc_egress_auth_bearer::{
    BearerEgressConfigBuilder, BearerEgressInterceptor, BearerSecret,
};

let config = BearerEgressConfigBuilder::new()
    .secret(BearerSecret::Hs256 { secret: secret_bytes })
    .issuer("svc-a")
    .audience("svc-b")
    .subject("svc-a-client")
    .lifetime_seconds(60)
    .build()?;
let interceptor = BearerEgressInterceptor::from_config(config);
```

The interceptor is then pushed into a `GrpcEgressInterceptorChain`. Attaching a chain to a client is
not possible through the public API of the transport crate yet; the builder of issue 16 adds
`with_interceptor`.

## Wire shape

| Header | Format |
|--------|--------|
| `authorization` | `Bearer <jwt>` |

## Constant-time secret comparison

`BearerSecret::ct_eq_hs256` uses `subtle::ConstantTimeEq` for raw secret material. Token content
comparisons are delegated to `jsonwebtoken`, which verifies the MAC in constant time.
