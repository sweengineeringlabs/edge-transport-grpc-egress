# grpc-egress-resilient

The resilience domain of the gRPC egress: the contracts, requests, responses, errors and the resilience
policy value (`ResilienceConfig`). No implementation.

The decorator that wraps a `GrpcEgress` in retry and a circuit breaker is in `grpc-egress-resilient-std`.
