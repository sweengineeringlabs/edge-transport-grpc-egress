//! Constant-time comparison of secrets, with `subtle`.

pub(crate) mod std_bearer_secret_guard;

pub(crate) use std_bearer_secret_guard::StdBearerSecretGuard;
