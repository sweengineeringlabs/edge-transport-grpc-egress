# ADR-006: Domain first: the transport splits into a domain crate and an implementation crate

**Status:** Accepted (the owner, 2026-10-16); implemented on `dev`
**Date:** 2026-10-16
**Tracking:** [#17](https://github.com/sweengineeringlabs/edge-transport-grpc-egress/issues/17) tasks 3, 4, 5, 7
**Amended by:** [ADR-010](ADR-010-programming-over-the-interface.md): the extension traits of the table below are domain traits implemented on `pub(crate)` types, not traits of the implementation crate.
**See also:** [ADR-004](ADR-004-remove-resilience-config-from-transport-primitive.md) (the transport knows nothing of retry and breaker)

---

## Context

One crate held what a gRPC egress call is (the trait, the request and response, the errors, the
options) and how it is made (`hyper`, `rustls`, `tower`, the load balancer, the security runtime).
A consumer that only needs the types built all of it. The `api/` source was almost clean already:
the coupling was in `Cargo.toml`, not in the code. What stopped a plain split was Rust: an inherent
`impl` must be in the crate of its type, and the crate declared types in `api/` and wrote their
methods in `core/`.

## Decision

Two crates, the domain first. `grpc-egress-transport-api` (`scm/main/transport-api`) holds the domain:
plain data, traits and errors, one folder per theme (`client`, `channel`, `interceptor`,
`loadbalancer`, `codec` behind the Cargo feature `prost`), each with the folders its kinds need
(`dto/`, `vo/`, `error/`, `traits/`). `grpc-egress-transport-std` (`scm/main/transport-std`) holds
the implementation and depends on the domain, never the other way round.

**Task 3: the types whose impl was in `core/`.** The rule applied: the domain crate has no
inherent `impl`. Data stays plain with public fields; the behaviour goes to the implementation
crate as an extension trait (a local trait implemented for the domain type, which Rust allows) or as
a type of the implementation crate. A caller keeps the call syntax by bringing the trait into scope.

| Type | Decision |
|---|---|
| `GrpcRequest` | (a) plain data; constructors and setters in the trait `GrpcRequestFactory` (`new`, `with_metadata`, `with_header`, `with_cancellation`); the struct builder `GrpcRequestBuilder` stays in std |
| `GrpcChannelConfig` | (a) plain data; `new`, `allow_plaintext` and the `with_*` setters in `GrpcChannelConfigFactory`; the struct builder `GrpcChannelConfigBuilder` stays in std. `Default` stays in the domain: a trait impl, not an inherent one, and the defaults are domain truth |
| `MtlsConfig` | (a) plain data; `new` and `with_root_ca` in `MtlsConfigFactory` |
| `CompressionMode` | (a) plain enum; the wire name in the trait `CompressionModeProvider` |
| `KeepAliveConfig` | (a) plain data; `Default` stays in the domain (the 10 s and 20 s of gRPC's recommendation) |
| `GrpcEgressInterceptorChain`, `TraceContextGrpcEgressInterceptor`, `ApplicationConfigBuilder`, `Conversions`, `TransportSvc` | (c) not domain: they are implementation, declared and implemented together in std |

**Superseded by ADR-010 (issue 60).** The extension traits above were declared in `std/`, which the owner's rule
forbids ("No new definitions in spi/std"): each is now a trait of the domain crate (`GrpcChannelConfigFactory` and the
setters are `GrpcChannelConfigBuilder`, `MtlsConfigFactory` stays in the domain with a request envelope, the
`*ConfigProvider`s are domain traits), implemented on a `pub(crate)` type and created by a factory of `saf/`. The
orphan rule is why: a `-std` crate cannot implement a domain trait for a domain type.

Rejected: (b) keeping the methods in the domain crate. It would make the domain crate carry
implementation, and it is the thing the audit rule `api_no_inherent_impl` flags.

**Task 4: `SecurityContext`.** `CallUnaryWithContextRequest` carries it, so the domain crate depends
on `edge-security-runtime` and re-exports `SecurityContext`. The dependency is light enough to
keep: `cargo tree -p grpc-egress-transport-api` shows `edge-security-runtime` pulling in only
`edge-security-runtime-identity-port` and `thiserror`. None of `tonic`, `hyper`, `hyper-util`,
`hyper-rustls`, `rustls`, `tower`, `swe-edge-loadbalancer`, `swe-edge-configbuilder` is present
(acceptance criterion 1).

**Task 7: `spi/`.** The `hyper` client and the balanced client stay in `spi/` for now. The reason is
checkable: the client is a wrapper over a third-party library (`hyper`, `hyper-rustls`), the audit
rule `boundary_peer_isolation` keeps `std/` and `spi/` from importing each other, and the builder
constructs the client, so the builder is in `spi/` too. Issue 18 replaces this client with
`proto-svc`; the placement is revisited there, not twice.

**Removed with the move** (they made `api/` name implementation types, or tested only themselves):
seven provided trait methods that gave a type a "role" in a signature (`describe_compression`,
`default_conversions`, `default_config_builder`, `default_facade`, `default_request_builder`,
`describe_chain_len`, `describe_trace_source`) with their 21 tests, and two dead functions of the
client protocol (a compile check nothing called, a function that only silenced an unused import) with
their tests. The resilience port and its stub went in ADR-004's follow-up (it reported a permanently
closed circuit whatever happened).

## Consequences

- Breaking for a caller that named the old package `edge-transport-grpc-egress-transport`: the
  domain types come from `grpc-egress-transport-api`; the factories, builders and extension traits from
  `grpc-egress-transport-std`.
- A consumer that needs only the types depends on the light crate.
- `retry`, `breaker`, `resilient` and `auth-bearer` depend on the domain crate for `GrpcEgress` and the
  types; their tests and examples depend on the implementation crate for constructors and factories.
- The remaining sibling crates are split the same way in later changes of issue 17.
