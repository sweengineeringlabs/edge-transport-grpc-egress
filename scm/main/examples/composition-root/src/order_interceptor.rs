//! `OrderInterceptor` -- an interceptor that records when it ran.

use std::sync::{Arc, Mutex, PoisonError};

use grpc_egress_transport::{
    AfterCallRequest, GrpcEgressError, GrpcEgressInterceptor, GrpcRequest,
};

/// An interceptor that appends `before:<label>` and `after:<label>` to a shared log when it runs.
pub struct OrderInterceptor {
    label: &'static str,
    log: Arc<Mutex<Vec<String>>>,
}

impl OrderInterceptor {
    /// An interceptor called `label` that writes into `log`.
    pub fn new(label: &'static str, log: Arc<Mutex<Vec<String>>>) -> Self {
        Self { label, log }
    }
}

impl GrpcEgressInterceptor for OrderInterceptor {
    fn before_call(&self, _req: &mut GrpcRequest) -> Result<(), GrpcEgressError> {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(format!("before:{}", self.label));
        Ok(())
    }

    fn after_call(&self, _req: AfterCallRequest<'_>) -> Result<(), GrpcEgressError> {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(format!("after:{}", self.label));
        Ok(())
    }
}
