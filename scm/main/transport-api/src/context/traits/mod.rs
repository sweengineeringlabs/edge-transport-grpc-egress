//! `traits` of the `context` theme.

pub mod caller_identity;
pub mod grpc_caller_identity_factory;

pub use caller_identity::CallerIdentity;
pub use grpc_caller_identity_factory::GrpcCallerIdentityFactory;
