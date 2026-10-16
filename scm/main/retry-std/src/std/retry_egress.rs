//! [`GrpcEgress`] impl for [`GrpcRetryClient`].

use crate::std::{GrpcRetryClient, RetryDecision};

use std::time::Instant;

use futures::future::BoxFuture;
use grpc_egress_transport::{
    CallStreamRequest, GrpcEgress, GrpcEgressError, GrpcEgressResult, GrpcMessageStreamResponse,
    GrpcRequest, GrpcResponse, HealthCheckRequest,
};
use tracing::{debug, trace, warn};

use edge_transport_retry::{BackoffScheduler, DefaultJitterRng};

impl<T: GrpcEgress + Send + Sync + 'static> GrpcEgress for GrpcRetryClient<T> {
    fn call_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(self.run_with_retry(request))
    }

    fn call_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        self.inner.call_stream(req)
    }

    fn health_check(&self, req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        self.inner.health_check(req)
    }
}

impl<T: GrpcEgress + Send + Sync + 'static> GrpcRetryClient<T> {
    async fn run_with_retry(&self, request: GrpcRequest) -> GrpcEgressResult<GrpcResponse> {
        let started = Instant::now();
        let total_budget = request.deadline;
        let max_attempts = self.config.max_attempts;
        let rate_limit_max = self.config.rate_limit_max_attempts;
        let mut rng = DefaultJitterRng::from_clock();
        let mut standard_attempt = 0u32;
        let mut rate_lim_attempt = 0u32;
        let mut last_error: Option<GrpcEgressError> = None;

        for attempt in 0..max_attempts {
            let elapsed = started.elapsed();
            let remaining = match total_budget.checked_sub(elapsed) {
                Some(r) if !r.is_zero() => r,
                _ => {
                    warn!(
                        attempt,
                        elapsed_ms = elapsed.as_millis() as u64,
                        budget_ms = total_budget.as_millis() as u64,
                        "grpc-retry: deadline exhausted before attempt",
                    );
                    return Err(last_error.unwrap_or_else(|| {
                        GrpcEgressError::Timeout(
                            "deadline exhausted before retry could be issued".into(),
                        )
                    }));
                }
            };

            let mut req_for_attempt = request.clone();
            req_for_attempt.deadline = remaining;

            let result = self.inner.call_unary(req_for_attempt).await;
            let decision = RetryDecision::classify(&result);

            match decision {
                RetryDecision::Success => return result,
                RetryDecision::Terminal => {
                    debug!(
                        attempt,
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
                            attempt,
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

                    if !self.budget_allows_sleep(started, total_budget, sleep_for, attempt) {
                        break;
                    }
                    trace!(
                        attempt,
                        sleep_ms = sleep_for.as_millis() as u64,
                        "grpc-retry: standard backoff before next attempt",
                    );
                    tokio::time::sleep(sleep_for).await;
                }
                RetryDecision::RetryRateLimit => {
                    last_error = result.err();
                    let next_rl = rate_lim_attempt + 1;
                    if next_rl > rate_limit_max {
                        debug!(attempt, "grpc-retry: rate_limit_max_attempts reached",);
                        break;
                    }

                    let hint = last_error.as_ref().and_then(|e| {
                        if let GrpcEgressError::Status(_, msg) = e {
                            RetryDecision::parse_retry_after_hint(msg)
                        } else {
                            None
                        }
                    });

                    let sleep_for = BackoffScheduler::rate_limit_backoff(
                        self.config.as_ref(),
                        rate_lim_attempt,
                        hint,
                        rng.next_unit(),
                    );
                    rate_lim_attempt += 1;

                    if !self.budget_allows_sleep(started, total_budget, sleep_for, attempt) {
                        break;
                    }
                    trace!(
                        attempt,
                        sleep_ms = sleep_for.as_millis() as u64,
                        hint_present = hint.is_some(),
                        "grpc-retry: rate-limit backoff before next attempt",
                    );
                    tokio::time::sleep(sleep_for).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            GrpcEgressError::Internal(
                "grpc-retry: exhausted attempts with no recorded error".into(),
            )
        }))
    }

    fn budget_allows_sleep(
        &self,
        started: Instant,
        total_budget: std::time::Duration,
        sleep_for: std::time::Duration,
        attempt: u32,
    ) -> bool {
        let elapsed_after = started.elapsed();
        let fits = elapsed_after
            .checked_add(sleep_for)
            .is_some_and(|t| t < total_budget);
        if !fits {
            debug!(
                attempt,
                sleep_ms = sleep_for.as_millis() as u64,
                elapsed_ms = elapsed_after.as_millis() as u64,
                budget_ms = total_budget.as_millis() as u64,
                "grpc-retry: backoff would exceed deadline, abandoning",
            );
        }
        fits
    }
}
