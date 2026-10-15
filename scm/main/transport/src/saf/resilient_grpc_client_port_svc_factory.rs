//! Composition site for [`ResilientGrpcClientPort`] — one file per trait keeps wiring focused.

use std::sync::Arc;

use crate::api::{GrpcEgress, ResilientGrpcClientPort};
use crate::core::traits::default_resilient_grpc_client::DefaultResilientGrpcClientPort;

/// Factory for the default [`ResilientGrpcClientPort`].
pub struct ResilientGrpcClientPortFactory;

impl ResilientGrpcClientPortFactory {
    /// Wrap `inner` with the default (no-op circuit) [`ResilientGrpcClientPort`].
    pub fn create(inner: Arc<dyn GrpcEgress>) -> Arc<dyn ResilientGrpcClientPort> {
        Arc::new(DefaultResilientGrpcClientPort::new(inner))
    }
}
