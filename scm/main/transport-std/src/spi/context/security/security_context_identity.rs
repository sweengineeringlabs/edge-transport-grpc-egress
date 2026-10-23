//! `SecurityContextIdentity` -- a `SecurityContext` as a [`CallerIdentity`].

use edge_security_runtime::SecurityContext;
use grpc_egress_transport::CallerIdentity;

/// A [`SecurityContext`] of `edge-security-runtime`, offered as the interface the identity view knows. The token is
/// the caller's only when the context is authenticated.
pub struct SecurityContextIdentity<'a> {
    context: &'a SecurityContext,
}

impl<'a> From<&'a SecurityContext> for SecurityContextIdentity<'a> {
    /// The identity that `context` stands for.
    fn from(context: &'a SecurityContext) -> Self {
        Self { context }
    }
}

impl CallerIdentity for SecurityContextIdentity<'_> {
    fn trace_id(&self) -> Option<&str> {
        self.context.trace_id.as_deref()
    }

    fn tenant_id(&self) -> Option<&str> {
        self.context.tenant_id.as_deref()
    }

    fn bearer_token(&self) -> Option<&str> {
        if self.context.authenticated {
            self.context.token.as_deref()
        } else {
            None
        }
    }

    fn headers(&self) -> Vec<(&str, &str)> {
        let mut entries: Vec<(&str, &str)> = self
            .context
            .metadata
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        entries.sort_unstable();
        entries
    }
}
