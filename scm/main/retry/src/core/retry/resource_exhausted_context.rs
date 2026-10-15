//! `impl` block for [`ResourceExhaustedContext`]. The type *declaration*
//! lives in `api/`.

use crate::api::ResourceExhaustedContext;

impl ResourceExhaustedContext {
    /// Classify a `RESOURCE_EXHAUSTED` grpc-message into a context.
    pub fn classify(message: &str) -> Self {
        let msg = message.to_ascii_lowercase();
        if msg.contains("quota") || msg.contains("billing") || msg.contains("plan limit") {
            ResourceExhaustedContext::HardQuota
        } else if msg.contains("rate")
            || msg.contains("too many requests")
            || msg.contains("throttl")
        {
            ResourceExhaustedContext::RateLimit
        } else {
            ResourceExhaustedContext::Capacity
        }
    }
}
