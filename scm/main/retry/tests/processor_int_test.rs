//! Integration tests for the `Processor` trait contract in `edge-transport-grpc-egress-retry`.

use edge_transport_grpc_egress_retry::{Error, GrpcRetryConfig, Processor, ProcessorRequest};

struct AlwaysOk;

impl Processor for AlwaysOk {
    fn validate(&self, _req: ProcessorRequest) -> Result<(), Error> {
        Ok(())
    }
}

struct AlwaysErr;

impl Processor for AlwaysErr {
    fn validate(&self, _req: ProcessorRequest) -> Result<(), Error> {
        Err(Error::InvalidConfig("always fails".into()))
    }
}

/// @covers: Processor — trait is implementable for both Ok and Err outcomes
#[test]
fn retry_trait_processor_custom_impl_accepts_config_int_test() {
    let config = GrpcRetryConfig::default();
    assert!(AlwaysOk
        .validate(ProcessorRequest {
            config: config.clone()
        })
        .is_ok());
    assert!(matches!(
        AlwaysErr.validate(ProcessorRequest { config }),
        Err(Error::InvalidConfig(_))
    ));
}

/// @covers: Processor — trait is object-safe
#[test]
fn retry_trait_processor_is_object_safe_int_test() {
    fn _assert(_: &dyn Processor) {}
}
