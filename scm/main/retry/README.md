# grpc-egress-retry

The retry domain of the gRPC egress: the contracts (`BackoffScheduler`, `RetryDecorator`,
`RetryInspector`, ...), their requests and responses, the errors and the retry policy value
(`GrpcRetryConfig`). No implementation.

The decorator that wraps a `GrpcEgress` with retry is in `grpc-egress-retry-std`.
