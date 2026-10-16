//! `TonicLbGrpcEgress` — spreads calls over several backends with the engine of one endpoint.

use std::future::Future;
use std::sync::Arc;

use futures::future::BoxFuture;
use swe_edge_loadbalancer::{
    BackendConfig, BackendPoolInstance, LoadbalancerConfig, LoadbalancerSvc, Outcome, Strategy,
};

use crate::spi::client::tonic::TonicGrpcEgress;
use grpc_egress_transport::{
    BalancingStrategy, CallStreamRequest, GrpcChannelConfigError, GrpcEgress, GrpcEgressError,
    GrpcEgressResult, GrpcMessageStreamResponse, GrpcRequest, GrpcResponse, HealthCheckRequest,
    LoadbalancerSettings,
};

/// A [`GrpcEgress`] that sends each call to a backend picked from a pool.
///
/// The call itself is made by a copy of the template client pointed at the picked backend, so
/// every option of the template (message cap, compression, request timeout, TLS, keep-alive,
/// interceptors) applies to a balanced call exactly as to a single-endpoint one. The outcome of
/// the call is reported to the pool: a backend that cannot be reached is avoided until a call to
/// it succeeds.
pub(crate) struct TonicLbGrpcEgress {
    template: TonicGrpcEgress,
    pool: Arc<BackendPoolInstance>,
}

impl TonicLbGrpcEgress {
    /// A balanced client over the backends of `settings`, calling through `template`.
    ///
    /// # Errors
    ///
    /// `InvalidOptionValue` for `loadbalancer` when the pool cannot be built from the settings.
    pub(crate) fn create(
        template: TonicGrpcEgress,
        settings: &LoadbalancerSettings,
    ) -> Result<Self, GrpcChannelConfigError> {
        let config = LoadbalancerConfig {
            strategy: match settings.strategy {
                BalancingStrategy::RoundRobin => Strategy::RoundRobin,
                BalancingStrategy::Weighted => Strategy::Weighted,
                BalancingStrategy::LeastConnections => Strategy::LeastConnections,
            },
            backends: settings
                .backends
                .iter()
                .map(|b| BackendConfig {
                    url: b.url.clone(),
                    weight: b.weight,
                })
                .collect(),
        };
        let pool = LoadbalancerSvc::build_pool(config).map_err(|e| {
            GrpcChannelConfigError::InvalidOptionValue {
                option: "loadbalancer",
                reason: e.to_string(),
            }
        })?;
        Ok(Self {
            template,
            pool: Arc::new(pool),
        })
    }

    /// Pick a backend, make the call through a client pointed at it, report how it went.
    async fn route<T, F, Fut>(&self, call: F) -> GrpcEgressResult<T>
    where
        F: FnOnce(TonicGrpcEgress) -> Fut,
        Fut: Future<Output = GrpcEgressResult<T>>,
    {
        let backend = LoadbalancerSvc::select(&self.pool)
            .map_err(|e| GrpcEgressError::Unavailable(e.to_string()))?;
        let result = call(self.template.retarget(&backend.url)).await;
        LoadbalancerSvc::report_outcome(&self.pool, &backend.id, Self::judge_outcome(&result));
        result
    }

    /// A backend is at fault when it cannot be reached or does not answer in time. Any other
    /// answer, an error status included, shows that it is up; so does a call the caller
    /// cancelled or that failed on this side.
    fn judge_outcome<T>(result: &GrpcEgressResult<T>) -> Outcome {
        match result {
            Err(
                e @ (GrpcEgressError::ConnectionFailed(_)
                | GrpcEgressError::Timeout(_)
                | GrpcEgressError::Unavailable(_)),
            ) => Outcome::Failure {
                reason: e.to_string(),
            },
            _ => Outcome::Success,
        }
    }
}

impl GrpcEgress for TonicLbGrpcEgress {
    fn call_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(self.route(move |client| async move { client.call_unary(request).await }))
    }

    fn call_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        Box::pin(self.route(move |client| async move { client.call_stream(req).await }))
    }

    fn call_server_stream(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        Box::pin(self.route(move |client| async move { client.call_server_stream(request).await }))
    }

    fn call_client_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(self.route(move |client| async move { client.call_client_stream(req).await }))
    }

    fn call_bidi_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        Box::pin(self.route(move |client| async move { client.call_bidi_stream(req).await }))
    }

    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async move {
            LoadbalancerSvc::select(&self.pool)
                .map(|_| ())
                .map_err(|e| GrpcEgressError::Unavailable(e.to_string()))
        })
    }
}
