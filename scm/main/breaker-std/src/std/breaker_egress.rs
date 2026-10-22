//! [`Transport`] and [`RawTransport`] impls for [`GrpcBreakerClient`].

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
use std::future::Future;

use edge_transport_breaker::DefaultBreakerTransition;
use edge_transport_breaker_policy::{
    Admission, AdmitRequest, BreakerTransition, Outcome, RecordOutcomeRequest,
};
use grpc_egress_breaker::BREAKER_EGRESS_LOG_PREFIX;
use tracing::error;

use crate::std::failure_classifier::StdFailureClassifier;
use crate::std::GrpcBreakerClient;

impl<T> GrpcBreakerClient<T> {
    fn refuse(path: &str, code: StatusCode, reason: String) -> CallError {
        CallError::Transport {
            path: path.into(),
            code,
            reason,
        }
    }

    /// Ask the state machine whether a call may go out, and move it as it says.
    ///
    /// # Errors
    ///
    /// `Transport` with `Unavailable` when the circuit is open: the call is refused without reaching the wrapped
    /// client. `Transport` with `Internal` when the state machine fails, which it does not do today.
    async fn admit(&self, path: &str) -> Result<(), CallError> {
        let snapshot = *self.node.lock().await;
        let admitted = DefaultBreakerTransition
            .admit(AdmitRequest {
                state: snapshot.state,
                consecutive_failures: snapshot.consecutive_failures,
                consecutive_successes: snapshot.consecutive_successes,
                config: (*self.config).clone().into(),
            })
            .map_err(|e| {
                Self::refuse(
                    path,
                    StatusCode::Internal,
                    format!("{BREAKER_EGRESS_LOG_PREFIX}: admit failed unexpectedly: {e}"),
                )
            })?;
        {
            let mut node = self.node.lock().await;
            node.state = admitted.state;
            node.consecutive_failures = admitted.consecutive_failures;
            node.consecutive_successes = admitted.consecutive_successes;
        }
        match admitted.admission {
            Admission::RejectOpen => Err(Self::refuse(
                path,
                StatusCode::Unavailable,
                format!("{BREAKER_EGRESS_LOG_PREFIX}: circuit open, request short-circuited"),
            )),
            Admission::Proceed => Ok(()),
        }
    }

    /// Tell the state machine how a call that went out ended.
    ///
    /// The call has already happened and its result is the true outcome, so a state machine that fails here
    /// (it does not today) is logged and the result of the call stands: a response that arrived is never
    /// replaced by a bookkeeping fault.
    async fn record(&self, outcome: Outcome) {
        let snapshot = *self.node.lock().await;
        match DefaultBreakerTransition.record(RecordOutcomeRequest {
            state: snapshot.state,
            consecutive_failures: snapshot.consecutive_failures,
            consecutive_successes: snapshot.consecutive_successes,
            config: (*self.config).clone().into(),
            outcome,
        }) {
            Ok(recorded) => {
                let mut node = self.node.lock().await;
                node.state = recorded.state;
                node.consecutive_failures = recorded.consecutive_failures;
                node.consecutive_successes = recorded.consecutive_successes;
            }
            Err(e) => error!(
                "{BREAKER_EGRESS_LOG_PREFIX}: record failed unexpectedly, the breaker did not move: {e}"
            ),
        }
    }

    /// Admit a call, make it, and record how it ended. `call` is not run when the circuit refuses: nothing is
    /// sent, and the wrapped client is not touched.
    async fn guard<R: Send, Fut: Future<Output = Result<R, CallError>> + Send>(
        &self,
        path: &str,
        call: impl FnOnce() -> Fut + Send,
    ) -> Result<R, CallError> {
        self.admit(path).await?;
        let result = call().await;
        let outcome = StdFailureClassifier::classify_result(&result);
        self.record(outcome).await;
        result
    }
}

impl<T: Transport + RawTransport> Transport for GrpcBreakerClient<T> {
    type Metadata = <T as Transport>::Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        self.guard(M::PATH, move || self.inner.call::<M>(request))
            .await
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
        self.guard(M::PATH, move || async {
            self.inner.call_server_stream::<M>(request).await
        })
        .await
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        request: CallClientStreamRequest<S>,
    ) -> Result<CallClientStreamResponse<M::Response, <Self as Transport>::Metadata>, CallError>
    {
        self.guard(M::PATH, move || async {
            self.inner.call_client_stream::<M, S>(request).await
        })
        .await
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
        self.guard(M::PATH, move || async {
            self.inner.call_bidi_stream::<M, S>(request).await
        })
        .await
    }
}

impl<T: Transport + RawTransport> RawTransport for GrpcBreakerClient<T> {
    type Metadata = <T as RawTransport>::Metadata;

    async fn call_raw(
        &self,
        request: CallRawRequest,
    ) -> Result<CallRawResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, message, options) = request.into_parts();

        self.admit(path.get_path()).await?;
        let result = self
            .inner
            .call_raw(
                CallRawRequestBuilder::new()
                    .with_path(path)
                    .with_message(message)
                    .with_options(options)
                    .build(),
            )
            .await;
        self.record(StdFailureClassifier::classify_result(&result))
            .await;
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

        self.admit(path.get_path()).await?;
        let result = self
            .inner
            .call_raw_server_stream(
                CallRawServerStreamRequestBuilder::new()
                    .with_path(path)
                    .with_message(message)
                    .with_options(options)
                    .build(),
            )
            .await;
        self.record(StdFailureClassifier::classify_result(&result))
            .await;
        result
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawClientStreamRequest<S>,
    ) -> Result<CallRawClientStreamResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, requests, options) = request.into_parts();

        self.admit(path.get_path()).await?;
        let result = self
            .inner
            .call_raw_client_stream(
                CallRawClientStreamRequestBuilder::new()
                    .with_path(path)
                    .with_requests(requests)
                    .with_options(options)
                    .build(),
            )
            .await;
        self.record(StdFailureClassifier::classify_result(&result))
            .await;
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

        self.admit(path.get_path()).await?;
        let result = self
            .inner
            .call_raw_bidi_stream(
                CallRawBidiStreamRequestBuilder::new()
                    .with_path(path)
                    .with_requests(requests)
                    .with_options(options)
                    .build(),
            )
            .await;
        self.record(StdFailureClassifier::classify_result(&result))
            .await;
        result
    }
}
