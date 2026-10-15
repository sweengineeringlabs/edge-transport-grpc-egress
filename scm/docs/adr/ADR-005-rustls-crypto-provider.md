# ADR-005: Use `aws_lc_rs` as the rustls Cryptography Provider

**Status:** Accepted
**Date:** 2026-06-05
**Deciders:** phdsystems
**Note (2026-10-15):** renumbered from ADR-001: two records carried that number (this one and the security specialisations record). The client this record calls `TonicGrpcClient` is `TonicGrpcEgress` today, a private type built through `TransportConstruction`.

---

## Context

`rustls` (the TLS library used by `hyper-rustls` and `tonic`) requires a
process-level cryptography provider to be registered before any TLS operations
occur. If none is registered, rustls panics at runtime:

```
Could not automatically determine the process-level CryptoProvider from Rustls crate features.
```

Two providers are available:

| Provider | Maintained by | FIPS certified | Notes |
|----------|--------------|----------------|-------|
| `ring` | community | No | Older default; maintenance has stagnated |
| `aws_lc_rs` | AWS | Yes (140-3) | Active; based on AWS's fork of BoringSSL |

The registration call is:

```rust
let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
```

The return value is discarded (`let _ =`) because `install_default()` returns
`Err` if a provider is already registered — which is correct behaviour, not an
error.

---

## Decision

Use `aws_lc_rs` as the process-level rustls cryptography provider.

Register it defensively at every entry point that constructs a TLS connector:
`TonicGrpcClient::new()` and `TonicGrpcClient::with_timeout()`. Both paths
build the same `hyper_rustls::HttpsConnectorBuilder` chain and both must
register the provider before doing so.

---

## Rationale

- **FIPS 140-3 compliance** — enterprise and regulated deployments require it.
  `ring` has no FIPS certification path; `aws_lc_rs` does.
- **Active maintenance** — AWS actively patches and updates `aws_lc_rs`.
  `ring`'s maintenance cadence has slowed significantly.
- **Same API surface** — switching providers requires no code changes beyond
  the `install_default()` call site.
- **Defensive registration** — calling `install_default()` at every constructor
  is idempotent (subsequent calls are no-ops) and eliminates the class of
  runtime panics caused by callers that skip the registration step.

---

## Consequences

- The `aws-lc-rs` crate is a build dependency. It requires a C compiler and
  cmake at build time (provided by the existing CI toolchain).
- If a downstream consumer explicitly registers `ring` first, our
  `install_default()` calls become no-ops and `ring` takes precedence — this
  is intentional and correct.
- Any future TLS entry point added to this crate must include the
  `install_default()` guard before constructing an `HttpsConnectorBuilder`.
