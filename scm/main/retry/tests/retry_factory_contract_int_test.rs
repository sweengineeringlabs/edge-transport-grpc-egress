//! The contract of `GrpcRetryFactory`: an implementor takes the client to wrap in the request value and
//! returns a `GrpcEgress`; the client of the caller keeps answering through it.

use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

use futures::future::BoxFuture;
use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryFactory};
use grpc_egress_transport::{
    GrpcEgress, GrpcEgressResult, GrpcRequest, GrpcResponse, HealthCheckRequest,
};

/// A client that answers every call with a body naming the method it was asked for.
struct EchoMethod;

impl GrpcEgress for EchoMethod {
    fn call_unary(&self, req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async move {
            Ok(GrpcResponse {
                body: req.method.into_bytes(),
                metadata: HashMap::new(),
            })
        })
    }

    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

/// A factory that wraps nothing: the product is the client it was given.
struct PassThroughFactory;

impl GrpcRetryFactory for PassThroughFactory {
    fn build<T: GrpcEgress + Send + Sync + 'static>(
        &self,
        req: BuildGrpcRetryRequest<T>,
    ) -> impl GrpcEgress + use<T> {
        req.inner
    }
}

fn request(method: &str) -> GrpcRequest {
    GrpcRequest {
        method: method.to_string(),
        body: Vec::new(),
        metadata: HashMap::new(),
        deadline: Duration::from_secs(1),
        cancellation: None,
    }
}

/// @covers: GrpcRetryFactory::build
#[tokio::test]
async fn test_build_returns_a_client_that_answers_through_the_factory() -> Result<(), Box<dyn Error>>
{
    let client = PassThroughFactory.build(BuildGrpcRetryRequest { inner: EchoMethod });

    let response = client.call_unary(request("pkg.Service/First")).await?;

    assert_eq!(response.body, b"pkg.Service/First");
    Ok(())
}

/// @covers: GrpcRetryFactory::build
#[tokio::test]
async fn test_build_called_twice_returns_independent_clients() -> Result<(), Box<dyn Error>> {
    let first = PassThroughFactory.build(BuildGrpcRetryRequest { inner: EchoMethod });
    let second = PassThroughFactory.build(BuildGrpcRetryRequest { inner: EchoMethod });

    let from_first = first.call_unary(request("pkg.Service/A")).await?;
    let from_second = second.call_unary(request("pkg.Service/B")).await?;

    assert_eq!(from_first.body, b"pkg.Service/A");
    assert_eq!(from_second.body, b"pkg.Service/B");
    Ok(())
}
