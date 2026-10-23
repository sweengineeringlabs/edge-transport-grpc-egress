//! [`Transport`] and [`RawTransport`] for [`GrpcRetryClient`].

use proto_pattern::{
    BidiStreamMethod, CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallOptions, CallRawBidiStreamRequest,
    CallRawBidiStreamRequestBuilder, CallRawBidiStreamResponse, CallRawClientStreamRequest,
    CallRawClientStreamRequestBuilder, CallRawClientStreamResponse, CallRawRequest,
    CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequest,
    CallRawServerStreamRequestBuilder, CallRawServerStreamResponse, CallRequest, CallResponse,
    CallServerStreamRequest, CallServerStreamResponse, ClientStreamMethod, MessageStream, Payload,
    RawTransport, ResponseStream, ServerStreamMethod, StatusCode, Transport, UnaryMethod,
};
use std::future::Future;
use std::time::{Duration, Instant};

use tracing::{debug, trace, warn};

use edge_transport_retry::{BackoffScheduler, DefaultJitterRng};

use grpc_egress_retry_api::{RetryClassifier, RetryDecision};

use super::grpc_retry_client::GrpcRetryClient;

impl<T: Transport + RawTransport, C: RetryClassifier + Send + Sync> Transport
    for GrpcRetryClient<T, C>
{
    type Metadata = <T as Transport>::Metadata;

    /// A typed unary call, retried. The request is encoded once, with its own [`Payload`], so that an attempt after
    /// the first can make the message again; the first attempt sends the message it was given. With one attempt
    /// allowed the call is forwarded and nothing is encoded.
    async fn call<M: UnaryMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        let (message, options) = request.into_parts();
        if self.config.max_attempts() <= 1 {
            return self
                .inner
                .call::<M>(CallRequest::new(message, options))
                .await;
        }
        let mut encoded = Vec::with_capacity(message.estimate_encoded_len());
        message
            .encode_into(&mut encoded)
            .map_err(CallError::Encode)?;

        let mut first = Some(message);
        self.run_with_retry(M::PATH, options.get_timeout(), |attempt_timeout| {
            let message = first.take();
            let encoded = &encoded;
            let metadata = options.get_metadata().clone();
            async move {
                let message = match message {
                    Some(message) => message,
                    None => <M::Request as Payload>::decode(encoded).map_err(CallError::Decode)?,
                };
                self.inner
                    .call::<M>(CallRequest::new(
                        message,
                        CallOptions::create(attempt_timeout, metadata),
                    ))
                    .await
            }
        })
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
        self.inner.call_server_stream::<M>(request).await
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        request: CallClientStreamRequest<S>,
    ) -> Result<CallClientStreamResponse<M::Response, <Self as Transport>::Metadata>, CallError>
    {
        self.inner.call_client_stream::<M, S>(request).await
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
        self.inner.call_bidi_stream::<M, S>(request).await
    }
}

impl<T: Transport + RawTransport, C: RetryClassifier + Send + Sync> RawTransport
    for GrpcRetryClient<T, C>
{
    type Metadata = <T as RawTransport>::Metadata;

    /// A raw unary call, retried: the bytes of the request are sent again on each attempt.
    async fn call_raw(
        &self,
        request: CallRawRequest,
    ) -> Result<CallRawResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, message, options) = request.into_parts();
        let body = message;
        self.run_with_retry(path.get_path(), options.get_timeout(), |attempt_timeout| {
            let path = path.clone();
            let body = body.clone();
            let metadata = options.get_metadata().clone();
            async move {
                self.inner
                    .call_raw(
                        CallRawRequestBuilder::new()
                            .with_path(path)
                            .with_message(body)
                            .with_options(CallOptions::create(attempt_timeout, metadata))
                            .build(),
                    )
                    .await
            }
        })
        .await
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
        self.inner
            .call_raw_server_stream(
                CallRawServerStreamRequestBuilder::new()
                    .with_path(path)
                    .with_message(message)
                    .with_options(options)
                    .build(),
            )
            .await
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawClientStreamRequest<S>,
    ) -> Result<CallRawClientStreamResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, requests, options) = request.into_parts();
        self.inner
            .call_raw_client_stream(
                CallRawClientStreamRequestBuilder::new()
                    .with_path(path)
                    .with_requests(requests)
                    .with_options(options)
                    .build(),
            )
            .await
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
        self.inner
            .call_raw_bidi_stream(
                CallRawBidiStreamRequestBuilder::new()
                    .with_path(path)
                    .with_requests(requests)
                    .with_options(options)
                    .build(),
            )
            .await
    }
}

impl<T, C: RetryClassifier + Send + Sync> GrpcRetryClient<T, C> {
    /// Run `attempt` until it succeeds, fails for good, or the policy gives up.
    ///
    /// `attempt` is given the timeout the attempt may take: what is left of `total_budget`, or none when the call has
    /// no timeout. `path` names the call in a failure the loop makes itself.
    async fn run_with_retry<R, F, Fut>(
        &self,
        path: &str,
        total_budget: Option<Duration>,
        mut attempt: F,
    ) -> Result<R, CallError>
    where
        F: FnMut(Option<Duration>) -> Fut,
        Fut: Future<Output = Result<R, CallError>>,
    {
        let started = Instant::now();
        let max_attempts = self.config.max_attempts();
        let rate_limit_max = self.config.rate_limit_max_attempts();
        let mut rng = DefaultJitterRng::from_clock();
        let mut standard_attempt = 0u32;
        let mut rate_lim_attempt = 0u32;
        let mut last_error: Option<CallError> = None;

        for attempt_index in 0..max_attempts {
            let remaining = match total_budget {
                None => None,
                Some(budget) => match budget.checked_sub(started.elapsed()) {
                    Some(left) if !left.is_zero() => Some(left),
                    _ => {
                        warn!(
                            attempt = attempt_index,
                            elapsed_ms = started.elapsed().as_millis() as u64,
                            budget_ms = budget.as_millis() as u64,
                            "grpc-retry: deadline exhausted before attempt",
                        );
                        return Err(last_error.unwrap_or_else(|| CallError::Transport {
                            path: path.into(),
                            code: StatusCode::DeadlineExceeded,
                            reason: "deadline exhausted before retry could be issued".to_string(),
                        }));
                    }
                },
            };

            let result = attempt(remaining).await;
            let decision = self.classifier.classify(&result);

            match decision {
                RetryDecision::Success => return result,
                RetryDecision::Terminal => {
                    debug!(
                        attempt = attempt_index,
                        outcome = ?result.as_ref().err(),
                        "grpc-retry: terminal failure, surfacing to caller",
                    );
                    return result;
                }
                RetryDecision::Retry => {
                    last_error = result.err();
                    let next_attempt = standard_attempt + 1;
                    if next_attempt >= max_attempts {
                        debug!(
                            attempt = attempt_index,
                            "grpc-retry: max_attempts reached on standard track",
                        );
                        break;
                    }

                    let sleep_for = BackoffScheduler::next_backoff(
                        self.config.as_ref(),
                        standard_attempt,
                        rng.next_unit(),
                    );
                    standard_attempt += 1;

                    if !self.budget_allows_sleep(started, total_budget, sleep_for, attempt_index) {
                        break;
                    }
                    trace!(
                        attempt = attempt_index,
                        sleep_ms = sleep_for.as_millis() as u64,
                        "grpc-retry: standard backoff before next attempt",
                    );
                    tokio::time::sleep(sleep_for).await;
                }
                RetryDecision::RetryRateLimit => {
                    last_error = result.err();
                    let next_rl = rate_lim_attempt + 1;
                    if next_rl > rate_limit_max {
                        debug!(
                            attempt = attempt_index,
                            "grpc-retry: rate_limit_max_attempts reached",
                        );
                        break;
                    }

                    let hint = last_error
                        .as_ref()
                        .and_then(|error| self.classifier.read_retry_after(error));

                    let sleep_for = BackoffScheduler::rate_limit_backoff(
                        self.config.as_ref(),
                        rate_lim_attempt,
                        hint,
                        rng.next_unit(),
                    );
                    rate_lim_attempt += 1;

                    if !self.budget_allows_sleep(started, total_budget, sleep_for, attempt_index) {
                        break;
                    }
                    trace!(
                        attempt = attempt_index,
                        sleep_ms = sleep_for.as_millis() as u64,
                        hint_present = hint.is_some(),
                        "grpc-retry: rate-limit backoff before next attempt",
                    );
                    tokio::time::sleep(sleep_for).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| CallError::Transport {
            path: path.into(),
            code: StatusCode::Internal,
            reason: "grpc-retry: exhausted attempts with no recorded error".to_string(),
        }))
    }

    /// `true` when a pause of `sleep_for` still fits in the total budget; always when the call has none.
    fn budget_allows_sleep(
        &self,
        started: Instant,
        total_budget: Option<Duration>,
        sleep_for: Duration,
        attempt: u32,
    ) -> bool {
        let Some(budget) = total_budget else {
            return true;
        };
        let elapsed_after = started.elapsed();
        let fits = elapsed_after
            .checked_add(sleep_for)
            .is_some_and(|t| t < budget);
        if !fits {
            debug!(
                attempt,
                sleep_ms = sleep_for.as_millis() as u64,
                elapsed_ms = elapsed_after.as_millis() as u64,
                budget_ms = budget.as_millis() as u64,
                "grpc-retry: backoff would exceed deadline, abandoning",
            );
        }
        fits
    }
}
