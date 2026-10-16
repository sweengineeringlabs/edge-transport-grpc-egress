# grpc-egress-breaker

The circuit-breaker domain of the gRPC egress: the contracts (`BreakerDecorator`, `BreakerObservable`,
...), their requests and responses, the errors and the breaker policy value (`GrpcBreakerConfig`).
No implementation.

The decorator that wraps a `GrpcEgress` with a circuit breaker is in `grpc-egress-breaker-std`.
