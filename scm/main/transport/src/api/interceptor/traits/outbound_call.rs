//! `OutboundCall` -- the call an interceptor is about to let go, as the interceptor may see and change it.

use std::time::Duration;

use proto_pattern::CallError;

/// A call that has not been sent: its path, its timeout and the request headers it will carry. An interceptor reads it
/// and changes it through this interface only, and the call is sent with the result; the type behind it is the
/// implementation's.
pub trait OutboundCall {
    /// The path of the method, `/package.Service/Method`.
    fn path(&self) -> &str;

    /// The timeout the call carries, when it has one.
    fn timeout(&self) -> Option<Duration>;

    /// Give the call `timeout`, or none.
    fn set_timeout(&mut self, timeout: Option<Duration>);

    /// `true` when the call already carries a header named `key`.
    fn has_header(&self, key: &str) -> bool;

    /// The values of the headers named `key`, in the order they were added.
    fn header_values(&self, key: &str) -> Vec<&str>;

    /// Add the header `key: value` after the ones the call carries.
    ///
    /// # Errors
    ///
    /// [`CallError::Transport`] with `InvalidArgument` when `key` or `value` is not valid metadata: the caller's own
    /// input is refused, so a retry policy must not retry it.
    fn add_header(&mut self, key: &str, value: &str) -> Result<(), CallError>;
}
