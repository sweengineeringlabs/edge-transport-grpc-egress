# grpc-egress-transport

The gRPC egress domain: the builder of a client and the options it takes, the interceptor that runs around a call,
and the factories of the views a client is wrapped in. The client is a `proto-pattern` `Transport` and `RawTransport`:
its calls, requests, responses, errors, status codes and metadata are the contract's. Plain data and traits; no
implementation and no heavy dependency (no `tonic`, `hyper`, `rustls`, `tower`, load balancer).

The implementation is `grpc-egress-transport-std`: a client built through
`TonicGrpcEgressBuilderSpiFactory`, with the builders and constructors for these types.

Themes: `client`, `channel`, `interceptor`, `loadbalancer`, `cancellation` and `context`.
