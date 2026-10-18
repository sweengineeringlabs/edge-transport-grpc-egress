//! `ContextHeaders` -- the headers an identity asks a call to carry.

use grpc_egress_transport::CallerIdentity;

/// The headers derived from a [`CallerIdentity`].
pub(crate) struct ContextHeaders;

impl ContextHeaders {
    /// `x-trace-id` and `x-tenant-id` when the identity has them, `authorization: Bearer <token>` when it holds a token to
    /// send, then the further headers it lists, in the order it lists them.
    pub(crate) fn derive(identity: &dyn CallerIdentity) -> Vec<(String, String)> {
        let mut headers = Vec::new();
        if let Some(trace_id) = identity.get_trace_id() {
            headers.push(("x-trace-id".to_string(), trace_id.to_string()));
        }
        if let Some(tenant_id) = identity.get_tenant_id() {
            headers.push(("x-tenant-id".to_string(), tenant_id.to_string()));
        }
        if let Some(token) = identity.get_bearer_token() {
            headers.push(("authorization".to_string(), format!("Bearer {token}")));
        }
        headers.extend(
            identity
                .list_headers()
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        headers
    }
}
