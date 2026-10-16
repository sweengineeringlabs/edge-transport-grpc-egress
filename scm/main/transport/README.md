# grpc-egress-transport

The gRPC egress domain: the `GrpcEgress` trait, the requests and responses it carries, its errors
and the options of a client. Plain data and traits; no implementation and no heavy dependency (no
`tonic`, `hyper`, `rustls`, `tower`, load balancer).

The implementation is `grpc-egress-transport-std`: a client built through
`TonicGrpcEgressBuilderSpiFactory`, with the builders and constructors for these types.

Themes: `client`, `channel`, `interceptor`, `loadbalancer` and `codec` (Cargo feature `prost`).
