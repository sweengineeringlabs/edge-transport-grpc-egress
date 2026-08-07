//! [`GrpcEgress`] impl for [`GrpcBreakerClient`].

use edge_transport_grpc_egress::{
    CallStreamRequest, GrpcEgress, GrpcEgressError, GrpcEgressResult, GrpcMessageStreamResponse,
    GrpcRequest, GrpcResponse, HealthCheckRequest,
};
use futures::future::BoxFuture;

use edge_transport_breaker::DefaultBreakerTransition;
use edge_transport_breaker_policy::{
    Admission, AdmitRequest, BreakerTransition, RecordOutcomeRequest,
};

use crate::api::{GrpcBreakerClient, BREAKER_EGRESS_LOG_PREFIX};
use crate::core::breaker::failure_classifier::DefaultFailureClassifier;

impl<T: GrpcEgress + Send + Sync + 'static> GrpcEgress for GrpcBreakerClient<T> {
    fn call_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async move {
            let node_snapshot = *self.node.lock().await;
            let admit_resp = match DefaultBreakerTransition.admit(AdmitRequest {
                state: node_snapshot.state,
                consecutive_failures: node_snapshot.consecutive_failures,
                consecutive_successes: node_snapshot.consecutive_successes,
                config: (*self.config).clone().into(),
            }) {
                Ok(resp) => resp,
                // BreakerDomainError is never actually constructed by admit();
                // this branch exists only because the trait's signature is
                // honest about the general Result contract.
                Err(e) => {
                    return Err(GrpcEgressError::Internal(format!(
                        "{BREAKER_EGRESS_LOG_PREFIX}: admit failed unexpectedly: {e}"
                    )))
                }
            };
            {
                let mut node = self.node.lock().await;
                node.state = admit_resp.state;
                node.consecutive_failures = admit_resp.consecutive_failures;
                node.consecutive_successes = admit_resp.consecutive_successes;
            }

            match admit_resp.admission {
                Admission::RejectOpen => Err(GrpcEgressError::Unavailable(format!(
                    "{BREAKER_EGRESS_LOG_PREFIX}: circuit open, request short-circuited"
                ))),
                Admission::Proceed => {
                    let result = self.inner.call_unary(request).await;
                    let outcome = match DefaultFailureClassifier::classify_result(&result) {
                        Ok(outcome) => outcome,
                        // BreakerDomainError is never actually constructed by
                        // classify(); this branch exists only because the
                        // trait's signature is honest about the general
                        // Result contract.
                        Err(e) => {
                            return Err(GrpcEgressError::Internal(format!(
                                "{BREAKER_EGRESS_LOG_PREFIX}: classify failed unexpectedly: {e}"
                            )))
                        }
                    };
                    let node_snapshot = *self.node.lock().await;
                    let record_resp = match DefaultBreakerTransition.record(RecordOutcomeRequest {
                        state: node_snapshot.state,
                        consecutive_failures: node_snapshot.consecutive_failures,
                        consecutive_successes: node_snapshot.consecutive_successes,
                        config: (*self.config).clone().into(),
                        outcome,
                    }) {
                        Ok(resp) => resp,
                        Err(e) => {
                            return Err(GrpcEgressError::Internal(format!(
                                "{BREAKER_EGRESS_LOG_PREFIX}: record failed unexpectedly: {e}"
                            )))
                        }
                    };
                    {
                        let mut node = self.node.lock().await;
                        node.state = record_resp.state;
                        node.consecutive_failures = record_resp.consecutive_failures;
                        node.consecutive_successes = record_resp.consecutive_successes;
                    }
                    result
                }
            }
        })
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
