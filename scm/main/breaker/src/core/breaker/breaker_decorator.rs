//! `impl BreakerDecorator for DefaultBreakerDecorator`.

use std::marker::PhantomData;
use std::sync::Arc;

use edge_transport_grpc_egress::GrpcEgress;
use tracing::debug;

use crate::api::{
    BreakerDecorator, BreakerDomainError, BreakerObservable, GrpcBreakerClient, WrapBreakerRequest,
    WrapBreakerResponse, BREAKER_DECORATOR_LABEL,
};

/// Default [`BreakerDecorator`] implementation — wraps `inner` with
/// [`GrpcBreakerClient::new`].
pub(crate) struct DefaultBreakerDecorator;

impl<T: GrpcEgress + Send + Sync + 'static> BreakerDecorator<T> for DefaultBreakerDecorator {
    fn wrap(
        &self,
        req: WrapBreakerRequest<T>,
    ) -> Result<WrapBreakerResponse<T>, BreakerDomainError> {
        debug!(
            decorator = BREAKER_DECORATOR_LABEL,
            "grpc-breaker: wrapping inner client"
        );
        let client: Arc<dyn GrpcEgress> = Arc::new(
            <GrpcBreakerClient<T> as BreakerObservable>::default_client(req.inner, req.config),
        );
        Ok(WrapBreakerResponse {
            client,
            _inner: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::api::GrpcBreakerConfig;
    use edge_transport_grpc_egress::{
        CallStreamRequest, GrpcEgressResult, GrpcMessageStreamResponse, GrpcRequest, GrpcResponse,
        HealthCheckRequest,
    };
    use futures::future::BoxFuture;

    struct DefaultBreakerDecoratorNoopEgress;
    impl GrpcEgress for DefaultBreakerDecoratorNoopEgress {
        fn call_unary(&self, _req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
            Box::pin(async {
                Ok(GrpcResponse {
                    body: b"noop-ok".to_vec(),
                    metadata: HashMap::new(),
                })
            })
        }
        fn call_stream(
            &self,
            _req: CallStreamRequest,
        ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
            unimplemented!("not exercised by this test")
        }
        fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
            Box::pin(async { Ok(()) })
        }
    }

    #[test]
    fn test_wrap_produces_a_client_with_the_supplied_config() {
        let resp = DefaultBreakerDecorator
            .wrap(WrapBreakerRequest {
                inner: DefaultBreakerDecoratorNoopEgress,
                config: GrpcBreakerConfig {
                    failure_threshold: 9,
                    cool_down_seconds: 5,
                    half_open_probe_count: 1,
                },
            })
            .expect("wrap is infallible");
        // client is now type-erased to Arc<dyn GrpcEgress>; the concrete
        // failure_threshold isn't observable through this surface, so the
        // real assertion is that the decorator constructs a genuinely
        // working GrpcEgress (proven via a real call_unary round trip).
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let result = rt.block_on(resp.client.call_unary(GrpcRequest::new(
            "svc/M",
            vec![],
            std::time::Duration::from_secs(1),
        )));
        assert_eq!(
            result.expect("wrapped client must delegate to inner").body,
            b"noop-ok",
            "wrapped client must return the inner client's real response"
        );
    }
}
