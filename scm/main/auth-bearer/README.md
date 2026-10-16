# grpc-egress-auth-bearer

The bearer-auth domain of the gRPC egress: the outbound config (`BearerEgressConfig`), the secret
(`BearerSecret`), the claims (`JwtClaims`), the errors and the validation contract. No implementation.

The interceptor that signs a JWT and adds `authorization: Bearer <jwt>` to each outbound request is in
`grpc-egress-auth-bearer-std`.
