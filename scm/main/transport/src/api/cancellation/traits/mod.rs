//! `traits` of the `cancellation` theme.

pub mod cancellation_signal;
pub mod grpc_cancellation_factory;

pub use cancellation_signal::CancellationSignal;
pub use grpc_cancellation_factory::GrpcCancellationFactory;
