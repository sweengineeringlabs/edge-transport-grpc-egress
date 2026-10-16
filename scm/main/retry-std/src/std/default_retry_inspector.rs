//! `impl RetryInspector for DefaultRetryInspector`.

use tracing::trace;

use grpc_egress_retry::Error;
use grpc_egress_retry::RetryInspectRequest;
use grpc_egress_retry::RetryInspectResponse;
use grpc_egress_retry::RetryInspector;
use grpc_egress_retry::RETRY_EGRESS_LOG_TARGET;

/// Default [`RetryInspector`] implementation.
pub(crate) struct DefaultRetryInspector;

impl RetryInspector for DefaultRetryInspector {
    fn describe(&self, _req: RetryInspectRequest) -> Result<RetryInspectResponse, Error> {
        trace!(
            target: RETRY_EGRESS_LOG_TARGET,
            "grpc-retry: inspector described",
        );
        Ok(RetryInspectResponse {
            label: "grpc-retry-inspector",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: describe
    #[test]
    fn test_describe_returns_grpc_retry_inspector_label() {
        let resp = DefaultRetryInspector
            .describe(RetryInspectRequest)
            .expect("infallible");
        assert_eq!(resp.label, "grpc-retry-inspector");
    }
}
