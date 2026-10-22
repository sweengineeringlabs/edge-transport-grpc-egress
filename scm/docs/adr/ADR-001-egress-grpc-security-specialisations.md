# ADR-001: Egress gRPC Security — Local Contract

**Status:** Accepted  
**Date:** 2026-06-12  
**Governing ADRs:** [ADR-015](../../../../../docs/3-architecture/adr/ADR-015-security-layer-architecture.md) (rules R1–R7) · [ADR-016](../../../../../docs/3-architecture/adr/ADR-016-grpc-security-specialisations.md) (gRPC design decisions)

---

## Mandate

Attach outbound credentials (signed JWT or mTLS client cert) to gRPC channels and calls before they leave the process. Never verify inbound tokens or inspect caller identity.

---

## What Lives Here

| Item | Crate | Why not shared |
|------|-------|---------------|
| `BearerEgressConfig` + interceptor | auth-bearer | Tonic client middleware; mints + signs JWTs for outbound calls |
| `BearerSecret` (private_pem variant) | auth-bearer | Sign-only key material — see R6 in ADR-015 |
| `BearerAuthError` | auth-bearer | JWT signing and config failures |
| `MtlsConfig` | transport | Client cert + key + optional CA for `tonic::transport::Channel` |

## What Is Re-exported from Shared

| Re-export | Source |
|-----------|--------|
| `JwtClaims` | `swe_edge_security::JwtClaims` |
| TLS load/parse errors | `swe_edge_security::TlsConfigError` |
