//! `OrderInterceptor` -- an interceptor that records when it ran.

use std::sync::{Arc, Mutex, PoisonError};

use grpc_egress_transport_api::{GrpcEgressInterceptor, InboundReply, OutboundCall};
use proto_pattern::{CallError, CallEvent, Metadata};

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
    fn before_call(&self, _call: &mut dyn OutboundCall) -> Result<(), CallError> {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(format!("before:{}", self.label));
        Ok(())
    }

    fn after_call(&self, _reply: &InboundReply<'_>) -> Result<(), CallError> {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(format!("after:{}", self.label));
        Ok(())
    }

    fn on_event(&self, event: &CallEvent<'_, Metadata>) {
        let phase = match event {
            CallEvent::Started { .. } => "started",
            CallEvent::Opened { .. } => "opened",
            CallEvent::MessageRead { .. } => "message_read",
            CallEvent::Completed { .. } => "completed",
            CallEvent::Failed { .. } => "failed",
            CallEvent::Cancelled { .. } => "cancelled",
        };
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(format!("event:{phase}:{}", self.label));
    }
}
