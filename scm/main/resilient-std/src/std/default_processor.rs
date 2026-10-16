//! `impl Processor for GrpcResilientSvcProcessor` — satisfies `service_type = "processor"`.

use crate::std::GrpcResilientSvcProcessor;

use grpc_egress_resilient::ResilientTransportError;
use grpc_egress_resilient::{DescribeRequest, DescribeResponse, Processor};

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
