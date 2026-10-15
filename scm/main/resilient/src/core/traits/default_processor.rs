//! `impl Processor for GrpcResilientSvcProcessor` — satisfies `service_type = "processor"`.

use crate::api::ResilientTransportError;
use crate::api::{DescribeRequest, DescribeResponse, GrpcResilientSvcProcessor, Processor};

impl Processor for GrpcResilientSvcProcessor {
    fn describe(&self, _req: DescribeRequest) -> Result<DescribeResponse, ResilientTransportError> {
        Ok(DescribeResponse {
            label: "grpc-resilient",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: describe
    #[test]
    fn test_describe_returns_grpc_resilient_label() {
        let resp = GrpcResilientSvcProcessor
            .describe(DescribeRequest)
            .expect("infallible");
        assert_eq!(resp.label, "grpc-resilient");
    }
}
