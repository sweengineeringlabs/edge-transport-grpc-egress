//! `ContextHeaders` -- the headers an identity asks a call to carry.

use grpc_egress_transport::CallerIdentity;

/// The headers derived from a [`CallerIdentity`]: `x-trace-id` and `x-tenant-id` when the identity has them,
/// `authorization: Bearer <token>` when it holds a token to send, then the further headers it lists, in the order it
/// lists them.
pub(crate) struct ContextHeaders {
    /// The `(name, value)` pairs, in the order they are sent.
    pub(crate) entries: Vec<(String, String)>,
}

impl From<&dyn CallerIdentity> for ContextHeaders {
    fn from(identity: &dyn CallerIdentity) -> Self {
        let mut headers = Vec::new();
        if let Some(trace_id) = identity.trace_id() {
            headers.push(("x-trace-id".to_string(), trace_id.to_string()));
        }
        if let Some(tenant_id) = identity.tenant_id() {
            headers.push(("x-tenant-id".to_string(), tenant_id.to_string()));
        }
        if let Some(token) = identity.bearer_token() {
            headers.push(("authorization".to_string(), format!("Bearer {token}")));
        }
        headers.extend(
            identity
                .headers()
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        Self { entries: headers }
    }
}
