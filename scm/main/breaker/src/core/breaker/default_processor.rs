//! `impl Processor for GrpcBreakerSvc` — satisfies `service_type = "processor"`.

use crate::api::{
    BreakerDomainError, DescribeRequest, DescribeResponse, GrpcBreakerSvc, Processor,
};

impl Processor for GrpcBreakerSvc {
    fn describe(&self, _req: DescribeRequest) -> Result<DescribeResponse, BreakerDomainError> {
        const LABEL: &str = "grpc-breaker";
        Ok(DescribeResponse { label: LABEL })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_describe_returns_the_grpc_breaker_label() {
        let resp = GrpcBreakerSvc
            .describe(DescribeRequest)
            .expect("infallible");
        assert_eq!(resp.label, "grpc-breaker");
    }
}
