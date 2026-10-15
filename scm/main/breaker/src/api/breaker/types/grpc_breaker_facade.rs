//! Facade type composing this crate's default implementations into one
//! convenient entry point. Declaration lives here per SEA rule 160; the
//! inherent impl lives in `core::breaker::grpc_breaker_facade`.

/// Facade composing this crate's default trait implementations.
pub struct GrpcBreakerFacade;
