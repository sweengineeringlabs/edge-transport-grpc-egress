# ADR-002: Security Context Propagation — egress/grpc outbound context

**Status:** Accepted  
**Date:** 2026-06-12  
**Governing ADR:** [ADR-017](https://github.com/sweengineeringlabs/edge/blob/main/docs/3-architecture/adr/ADR-017-security-context-propagation.md) — Security Context Propagation Pipeline  
**See also:** [ADR-001](ADR-001-egress-grpc-security-specialisations.md) — egress gRPC security specialisations
**Note (2026-10-15):** names in this record changed after it was written: `GrpcOutbound` is `GrpcEgress`, and `call_with_context` is `call_unary_with_context`.

---

## Mandate

Add `call_with_context` to `GrpcOutbound`. Additive only — no breaking changes in this repo.

---

## `GrpcOutbound` change (additive)

```rust
pub trait GrpcOutbound: Send + Sync {
    async fn call(&self, req: GrpcRequest) -> Result<GrpcResponse, EgressError> {
        self.call_with_context(req, &SecurityContext::unauthenticated()).await
    }
    async fn call_with_context(
        &self,
        req: GrpcRequest,
        ctx: &SecurityContext,
    ) -> Result<GrpcResponse, EgressError>;
}
```

Existing callers using `call()` continue to work via the shim — no migration required. Per-tenant credential injection lands in ADR-018.

---

## Dependency change

```toml
edge-domain = { ..., features = ["security"] }
```

---

## What does not change

`BearerEgressConfig`, `BearerSecret` (private_pem), `MtlsConfig`, JWT signing logic — all unchanged.

---

## Cascade position

Step 7 of 11 (parallel with egress/http and ingress/security). Can proceed immediately after swe-edge-security ADR-001.
