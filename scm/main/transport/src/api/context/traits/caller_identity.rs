//! `CallerIdentity` -- who is calling, as the headers of a call need to know it.

/// The identity of the caller, as far as a call carries it: a trace id, a tenant, a bearer token and further headers.
///
/// The view of a client that carries an identity (`GrpcCallerIdentityFactory`) knows only this interface, so the
/// caller keeps the security context it already has and implements the trait for it.
pub trait CallerIdentity: Send + Sync {
    /// The id that ties the call to the request it serves, when there is one.
    fn get_trace_id(&self) -> Option<&str>;

    /// The tenant the caller acts for, when there is one.
    fn get_tenant_id(&self) -> Option<&str>;

    /// The bearer token to send: only when the caller is authenticated and holds one.
    fn get_bearer_token(&self) -> Option<&str>;

    /// Further headers to send, as `(name, value)`, in a stable order.
    fn list_headers(&self) -> Vec<(&str, &str)>;
}
