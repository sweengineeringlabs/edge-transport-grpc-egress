//! API of the `cancellation` theme: ending a call from the caller's side with a signal.

pub mod dto;
pub mod traits;

pub use dto::BuildGrpcCancellationRequest;
pub use traits::{CancellationSignal, GrpcCancellationFactory};
