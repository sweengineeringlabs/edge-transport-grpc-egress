//! `ContextHeaders` -- the headers a security context asks a call to carry.

use grpc_egress_transport::SecurityContext;

/// The headers derived from a [`SecurityContext`].
pub(crate) struct ContextHeaders;

impl ContextHeaders {
    /// `x-trace-id` and `x-tenant-id` when the context has them, `authorization: Bearer <token>` when it is
    /// authenticated and holds a token, then the entries of its `metadata`, in a stable order.
    pub(crate) fn derive(ctx: &SecurityContext) -> Vec<(String, String)> {
        let mut headers = Vec::new();
        if let Some(trace_id) = &ctx.trace_id {
            headers.push(("x-trace-id".to_string(), trace_id.clone()));
        }
        if let Some(tenant_id) = &ctx.tenant_id {
            headers.push(("x-tenant-id".to_string(), tenant_id.clone()));
        }
        if let (true, Some(token)) = (ctx.authenticated, &ctx.token) {
            headers.push(("authorization".to_string(), format!("Bearer {token}")));
        }
        let mut entries: Vec<_> = ctx.metadata.iter().collect();
        entries.sort();
        headers.extend(entries.into_iter().map(|(k, v)| (k.clone(), v.clone())));
        headers
    }
}
