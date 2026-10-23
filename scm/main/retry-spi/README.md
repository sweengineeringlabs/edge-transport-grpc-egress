# grpc-egress-retry-spi

Provider-backed retry decorator for any client implementing `Transport` and `RawTransport`, plus TOML readers for
retry policy. The decorator retries typed and raw unary calls with exponential backoff and jitter; streaming calls are
forwarded because they cannot be replayed.

The decorator obtains its retry policy through the domain `RetryClassifierFactory` contract, implemented by
[`grpc-egress-retry-std`](../retry-std/README.md). The SPI package owns its engine, runtime, and TOML dependencies.

See the [domain contract](../retry-api/README.md) and [ADR-007](../../docs/adr/ADR-007-decorators-wrap-a-built-client.md).
