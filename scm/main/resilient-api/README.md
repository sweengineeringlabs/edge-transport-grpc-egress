# grpc-egress-resilient-api

**Audience:** Rust developers configuring retry and breaker behavior together.

## WHAT

The resilience domain of the gRPC egress: the contracts, requests, responses, errors and the resilience
policy value (`ResilienceConfig`). No implementation.

The decorator that wraps a `Transport + RawTransport` client in retry and a circuit breaker is in `grpc-egress-resilient-spi`.

## WHY

One validated policy composes the independently owned retry and breaker policies.

## HOW

Use the domain interfaces described above and the standard factory for this crate.

## API

`ResilienceConfig` holds validated retry and breaker policies; the domain also declares the resilient factory and request contracts.

## Documentation

See the [SPI composition](../resilient-spi/README.md), [retry](../retry-api/README.md), and [breaker](../breaker-api/README.md) contracts.
