//! Reserved metadata keys used by the bearer interceptors.

/// Standard HTTP/2 / gRPC `authorization` metadata key (lower-case).
pub const AUTHORIZATION_HEADER: &str = "authorization";

/// Internal metadata key under which a successfully validated JWT
/// `sub` claim is republished by the bearer egress interceptor.
pub const EXTRACTED_BEARER_SUBJECT: &str = "x-edge-extracted-bearer-subject";
