//! `BalancedGrpcEgress` -- spreads calls over several backends, each with a client of its own.

use std::sync::Arc;

use futures::future::BoxFuture;
use proto_pattern::RawTransport;
use swe_edge_loadbalancer::{
    BackendConfig, BackendPoolInstance, LoadbalancerConfig, LoadbalancerSvc, Outcome, Strategy,
};

use crate::spi::client::protosvc::ProtoSvcGrpcEgress;
use grpc_egress_transport::{
    BalancingStrategy, CallStreamRequest, GrpcChannelConfigError, GrpcEgress, GrpcEgressError,
    GrpcEgressResult, GrpcMessageStreamResponse, GrpcRequest, GrpcResponse, GrpcStatusCode,
    HealthCheckRequest, LoadbalancerSettings,
};

/// A [`GrpcEgress`] that sends each call to a backend picked from a pool.
///
/// Every backend has a client built once, from the same options (message cap, compression, request
/// timeout, TLS, keep-alive, interceptors), so a balanced call is made exactly as a single-endpoint one
/// and each backend keeps its own connection. The outcome of the call is reported to the pool: a backend
/// that cannot be reached is avoided until a call to it succeeds.
pub(crate) struct BalancedGrpcEgress<T> {
    clients: Vec<(String, ProtoSvcGrpcEgress<T>)>,
    pool: Arc<BackendPoolInstance>,
}

impl<T> BalancedGrpcEgress<T> {
    /// A balanced client over the backends of `settings`; `clients` has the client of each backend by URL.
    ///
    /// # Errors
    ///
    /// `InvalidOptionValue` for `loadbalancer` when the pool cannot be built from the settings.
    pub(crate) fn create(
        clients: Vec<(String, ProtoSvcGrpcEgress<T>)>,
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
            clients,
            pool: Arc::new(pool),
        })
    }

    /// Pick a backend, make the call through its client, report how it went.
    async fn route<R, F>(&self, call: F) -> GrpcEgressResult<R>
    where
        F: for<'a> FnOnce(&'a ProtoSvcGrpcEgress<T>) -> BoxFuture<'a, GrpcEgressResult<R>>,
    {
        let backend = LoadbalancerSvc::select(&self.pool)
            .map_err(|e| GrpcEgressError::Unavailable(e.to_string()))?;
        let client = self
            .clients
            .iter()
            .find(|(url, _)| *url == backend.url)
            .map(|(_, client)| client)
            .ok_or_else(|| {
                tracing::warn!(url = %backend.url, "the pool picked a backend that has no client");
                GrpcEgressError::Internal("internal client error".into())
            })?;
        let result = call(client).await;
        LoadbalancerSvc::report_outcome(&self.pool, &backend.id, Self::judge_outcome(&result));
        result
    }

    /// A backend is at fault when it cannot be reached or does not answer in time. Any other answer, an
    /// error status included, shows that it is up; so does a call the caller cancelled or that failed on
    /// this side.
    fn judge_outcome<R>(result: &GrpcEgressResult<R>) -> Outcome {
        match result {
            Err(
                e @ (GrpcEgressError::ConnectionFailed(_)
                | GrpcEgressError::Timeout(_)
                | GrpcEgressError::Unavailable(_)
                | GrpcEgressError::Status(GrpcStatusCode::DeadlineExceeded, _)),
            ) => Outcome::Failure {
                reason: e.to_string(),
            },
            _ => Outcome::Success,
        }
    }
}

impl<T: RawTransport + Send + Sync + 'static> GrpcEgress for BalancedGrpcEgress<T> {
    fn call_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(self.route(move |client: &ProtoSvcGrpcEgress<T>| client.call_unary(request)))
    }

    fn call_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        Box::pin(self.route(move |client: &ProtoSvcGrpcEgress<T>| client.call_stream(req)))
    }

    fn call_server_stream(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        Box::pin(
            self.route(move |client: &ProtoSvcGrpcEgress<T>| client.call_server_stream(request)),
        )
    }

    fn call_client_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(self.route(move |client: &ProtoSvcGrpcEgress<T>| client.call_client_stream(req)))
    }

    fn call_bidi_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        Box::pin(self.route(move |client: &ProtoSvcGrpcEgress<T>| client.call_bidi_stream(req)))
    }

    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async move {
            LoadbalancerSvc::select(&self.pool)
                .map(|_| ())
                .map_err(|e| GrpcEgressError::Unavailable(e.to_string()))
        })
    }
}
