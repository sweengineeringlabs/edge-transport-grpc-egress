//! `EndpointTransport` -- one endpoint, or several backends behind a load balancer, each with a client of its own.

use proto_pattern::{
    BidiStreamMethod, CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallRawBidiStreamRequest, CallRawBidiStreamRequestBuilder,
    CallRawBidiStreamResponse, CallRawClientStreamRequest, CallRawClientStreamRequestBuilder,
    CallRawClientStreamResponse, CallRawRequest, CallRawRequestBuilder, CallRawResponse,
    CallRawServerStreamRequest, CallRawServerStreamRequestBuilder, CallRawServerStreamResponse,
    CallRequest, CallResponse, CallServerStreamRequest, CallServerStreamResponse,
    ClientStreamMethod, MessageStream, RawTransport, ResponseStream, ServerStreamMethod,
    StatusCode, Transport, UnaryMethod,
};
use std::sync::Arc;

use grpc_egress_transport::{BalancingStrategy, GrpcChannelConfigError, LoadbalancerSettings};
use swe_edge_loadbalancer::{
    Backend, BackendConfig, BackendPoolInstance, LoadbalancerConfig, LoadbalancerSvc, Outcome,
    Strategy,
};

/// The client of the endpoint, or of a backend the load balancer picks per call.
///
/// Every backend has a client built once, from the same options (message limits, compression, request timeout,
/// TLS, keep-alive), so a balanced call is made exactly as a single-endpoint one and each backend keeps its own
/// connection. The outcome of a call is reported to the pool: a backend that cannot be reached, or does not
/// answer in time, is avoided until a call to it succeeds. A typed call is forwarded as a typed call and a raw call as
/// a raw call.
pub(crate) struct EndpointTransport<T> {
    clients: Vec<(String, T)>,
    pool: Option<Arc<BackendPoolInstance>>,
}

/// The client a call goes to and, when a pool picked it, the backend to report the outcome to.
struct Pick<'a, T> {
    client: &'a T,
    backend: Option<Backend>,
}

impl<T> EndpointTransport<T> {
    /// An endpoint over `clients` (the client of each URL). With `balancing`, calls are spread over them; without it
    /// the first client takes every call.
    ///
    /// # Errors
    ///
    /// `InvalidOptionValue` for `loadbalancer` when the pool cannot be built from the settings.
    pub(crate) fn create(
        clients: Vec<(String, T)>,
        balancing: Option<&LoadbalancerSettings>,
    ) -> Result<Self, GrpcChannelConfigError> {
        let pool = match balancing {
            Some(settings) => Some(Arc::new(Self::create_pool(settings)?)),
            None => None,
        };
        Ok(Self { clients, pool })
    }

    fn create_pool(
        settings: &LoadbalancerSettings,
    ) -> Result<BackendPoolInstance, GrpcChannelConfigError> {
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
        LoadbalancerSvc::build_pool(config).map_err(|e| {
            GrpcChannelConfigError::InvalidOptionValue {
                option: "loadbalancer",
                reason: e.to_string(),
            }
        })
    }

    /// The client for the next call.
    fn pick(&self, path: &str) -> Result<Pick<'_, T>, CallError> {
        let Some(pool) = &self.pool else {
            return self
                .clients
                .first()
                .map(|(_, client)| Pick {
                    client,
                    backend: None,
                })
                .ok_or_else(|| {
                    Self::refuse(path, StatusCode::Internal, "the endpoint has no client")
                });
        };
        let backend = LoadbalancerSvc::select(pool)
            .map_err(|e| Self::refuse(path, StatusCode::Unavailable, &e.to_string()))?;
        let client = self
            .clients
            .iter()
            .find(|(url, _)| *url == backend.url)
            .map(|(_, client)| client)
            .ok_or_else(|| {
                Self::refuse(
                    path,
                    StatusCode::Internal,
                    "the pool picked a backend that has no client",
                )
            })?;
        Ok(Pick {
            client,
            backend: Some(backend),
        })
    }

    fn refuse(path: &str, code: StatusCode, reason: &str) -> CallError {
        CallError::Transport {
            path: path.into(),
            code,
            reason: reason.to_string(),
        }
    }

    /// Report how the call went to the pool, when a pool picked the backend.
    fn report<R>(&self, pick: &Pick<'_, T>, result: &Result<R, CallError>) {
        if let (Some(pool), Some(backend)) = (&self.pool, &pick.backend) {
            LoadbalancerSvc::report_outcome(pool, &backend.id, Self::judge_outcome(result));
        }
    }

    /// A backend is at fault when it cannot be reached or does not answer in time (a passed deadline is reported as
    /// a status, whoever noticed it, so it counts). Any other answer, an error status included, shows that it is up;
    /// so does a call that failed on this side.
    fn judge_outcome<R>(result: &Result<R, CallError>) -> Outcome {
        match result {
            Err(
                error @ (CallError::Transport {
                    code: StatusCode::Unavailable | StatusCode::Unknown,
                    ..
                }
                | CallError::Status {
                    code: StatusCode::DeadlineExceeded,
                    ..
                }),
            ) => Outcome::Failure {
                reason: error.to_string(),
            },
            _ => Outcome::Success,
        }
    }
}

impl<T: Transport + RawTransport> Transport for EndpointTransport<T> {
    type Metadata = <T as Transport>::Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        let pick = self.pick(M::PATH)?;
        let result = pick.client.call::<M>(request).await;
        self.report(&pick, &result);
        result
    }

    async fn call_server_stream<M: ServerStreamMethod>(
        &self,
        request: CallServerStreamRequest<M::Request>,
    ) -> Result<
        CallServerStreamResponse<
            impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>,
        >,
        CallError,
    > {
        let pick = self.pick(M::PATH)?;
        let result = pick.client.call_server_stream::<M>(request).await;
        self.report(&pick, &result);
        result
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        request: CallClientStreamRequest<S>,
    ) -> Result<CallClientStreamResponse<M::Response, <Self as Transport>::Metadata>, CallError>
    {
        let pick = self.pick(M::PATH)?;
        let result = pick.client.call_client_stream::<M, S>(request).await;
        self.report(&pick, &result);
        result
    }

    async fn call_bidi_stream<M: BidiStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        request: CallBidiStreamRequest<S>,
    ) -> Result<
        CallBidiStreamResponse<
            impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>,
        >,
        CallError,
    > {
        let pick = self.pick(M::PATH)?;
        let result = pick.client.call_bidi_stream::<M, S>(request).await;
        self.report(&pick, &result);
        result
    }
}

impl<T: Transport + RawTransport> RawTransport for EndpointTransport<T> {
    type Metadata = <T as RawTransport>::Metadata;

    async fn call_raw(
        &self,
        request: CallRawRequest,
    ) -> Result<CallRawResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, message, options) = request.into_parts();
        let pick = self.pick(path.get_path())?;
        let result = pick
            .client
            .call_raw(
                CallRawRequestBuilder::new()
                    .with_path(path)
                    .with_message(message)
                    .with_options(options)
                    .build(),
            )
            .await;
        self.report(&pick, &result);
        result
    }

    async fn call_raw_server_stream(
        &self,
        request: CallRawServerStreamRequest,
    ) -> Result<
        CallRawServerStreamResponse<
            impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>,
        >,
        CallError,
    > {
        let (path, message, options) = request.into_parts();
        let pick = self.pick(path.get_path())?;
        let result = pick
            .client
            .call_raw_server_stream(
                CallRawServerStreamRequestBuilder::new()
                    .with_path(path)
                    .with_message(message)
                    .with_options(options)
                    .build(),
            )
            .await;
        self.report(&pick, &result);
        result
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawClientStreamRequest<S>,
    ) -> Result<CallRawClientStreamResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, requests, options) = request.into_parts();

        let pick = self.pick(path.get_path())?;
        let result = pick
            .client
            .call_raw_client_stream(
                CallRawClientStreamRequestBuilder::new()
                    .with_path(path)
                    .with_requests(requests)
                    .with_options(options)
                    .build(),
            )
            .await;
        self.report(&pick, &result);
        result
    }

    async fn call_raw_bidi_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawBidiStreamRequest<S>,
    ) -> Result<
        CallRawBidiStreamResponse<
            impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>,
        >,
        CallError,
    > {
        let (path, requests, options) = request.into_parts();

        let pick = self.pick(path.get_path())?;
        let result = pick
            .client
            .call_raw_bidi_stream(
                CallRawBidiStreamRequestBuilder::new()
                    .with_path(path)
                    .with_requests(requests)
                    .with_options(options)
                    .build(),
            )
            .await;
        self.report(&pick, &result);
        result
    }
}
