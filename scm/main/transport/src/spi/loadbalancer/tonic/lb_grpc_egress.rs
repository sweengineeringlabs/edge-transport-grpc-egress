//! Tonic-backed gRPC egress with load-balancer health tracking.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use futures::future::BoxFuture;
use http_body_util::{BodyExt as _, Full};
use tower::ServiceExt as _;

use swe_edge_loadbalancer::{BackendPoolInstance, LoadbalancerConfig, LoadbalancerSvc, Outcome};

use crate::api::Conversions as StatusConversions;
use crate::api::{GrpcEgress, GrpcEgressError, GrpcEgressResult, GrpcRequest, GrpcResponse};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// A [`GrpcEgress`] backed by `tonic::transport::Channel::balance_list`.
///
/// Uses [`LoadbalancerConfig`] for backend registration and
/// [`BackendPoolInstance`] for health-aware selection and outcome reporting.
/// Each unary call selects a backend from the pool, routes the request
/// through tonic's built-in load-balancer channel, and reports success or
/// failure back to the pool so downstream calls can avoid degraded backends.
#[derive(Debug)]
pub(crate) struct TonicLbGrpcEgress {
    pub(crate) channel: tonic::transport::Channel,
    pub(crate) pool: Arc<BackendPoolInstance>,
    pub(crate) timeout: Duration,
}

impl TonicLbGrpcEgress {
    /// Construct a `TonicLbGrpcEgress` from a [`LoadbalancerConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`GrpcEgressError::Unavailable`] when:
    /// - The config has no backends.
    /// - Any backend URL is not a valid URI.
    pub(crate) fn from_config(config: LoadbalancerConfig) -> Result<Self, GrpcEgressError> {
        if config.backends.is_empty() {
            return Err(GrpcEgressError::Unavailable(
                "load-balancer config has no backends".to_string(),
            ));
        }

        let endpoints: Result<Vec<tonic::transport::Endpoint>, GrpcEgressError> = config
            .backends
            .iter()
            .map(|b| {
                tonic::transport::Endpoint::from_shared(b.url.clone()).map_err(|e| {
                    GrpcEgressError::Unavailable(format!("invalid backend URL '{}': {e}", b.url))
                })
            })
            .collect();
        let endpoints = endpoints?;

        // tonic 0.12: balance_list returns Channel directly (not a tuple).
        let channel = tonic::transport::Channel::balance_list(endpoints.into_iter());

        let pool = LoadbalancerSvc::build_pool(config)
            .map_err(|e| GrpcEgressError::Unavailable(e.to_string()))?;

        Ok(Self {
            channel,
            pool: Arc::new(pool),
            timeout: DEFAULT_TIMEOUT,
        })
    }

    /// Override the per-call timeout (default: 30 seconds).
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "only exercised in this crate's own tests; production wiring pending"
        )
    )]
    pub(crate) fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl GrpcEgress for TonicLbGrpcEgress {
    fn call_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        let pool = Arc::clone(&self.pool);
        let timeout = self.timeout;
        let channel = self.channel.clone();

        Box::pin(async move {
            // Health-check gate: fail fast when no healthy backend is available.
            let backend = LoadbalancerSvc::select(&pool)
                .map_err(|e| GrpcEgressError::Unavailable(e.to_string()))?;

            // URI = backend URL + method path (e.g. `http://host:50051/pkg.Svc/Method`).
            let method = request.method.trim_start_matches('/');
            let uri_str = format!("{}/{}", backend.url.trim_end_matches('/'), method);
            let uri: http::Uri = uri_str.parse().map_err(|e| {
                tracing::warn!(error = %e, uri = %uri_str, "invalid gRPC URI for LB call");
                GrpcEgressError::Internal("invalid gRPC target URI".to_string())
            })?;

            // Encode body as a gRPC data frame: 1 compression byte + 4-byte BE length.
            let payload: Bytes = request.body.into();
            let mut frame = BytesMut::with_capacity(5 + payload.len());
            frame.put_u8(0x00);
            frame.put_u32(payload.len() as u32);
            frame.put_slice(&payload);

            // Convert to tonic::body::BoxBody (UnsyncBoxBody<Bytes, tonic::Status>).
            let tonic_body: tonic::body::BoxBody = Full::new(frame.freeze())
                .map_err(|e: Infallible| match e {})
                .boxed_unsync();

            let mut builder = http::Request::builder()
                .method(http::Method::POST)
                .uri(uri)
                .header(http::header::CONTENT_TYPE, "application/grpc")
                .header("te", "trailers");

            for (k, v) in &request.metadata {
                builder = builder.header(k.as_str(), v.as_str());
            }

            let http_req = builder.body(tonic_body).map_err(|e| {
                tracing::warn!(error = %e, "failed to build gRPC request for LB call");
                GrpcEgressError::Internal("failed to construct gRPC request".to_string())
            })?;

            // Race the channel call against the per-call deadline.
            let call_result = tokio::time::timeout(timeout, channel.oneshot(http_req)).await;

            let outcome = match &call_result {
                Ok(Ok(_)) => Outcome::Success,
                _ => Outcome::Failure {
                    reason: "grpc call failed or timed out".to_string(),
                },
            };
            LoadbalancerSvc::report_outcome(&pool, &backend.id, outcome);

            let resp = call_result
                .map_err(|_| GrpcEgressError::Timeout("per-call deadline exceeded".to_string()))?
                .map_err(|e| GrpcEgressError::Unavailable(e.to_string()))?;

            let (parts, body) = resp.into_parts();

            let collected = body
                .collect()
                .await
                .map_err(|e| GrpcEgressError::Internal(e.to_string()))?;

            // Extract trailers into an owned map before consuming `collected`.
            let mut trailer_headers: HashMap<String, String> = HashMap::new();
            if let Some(trailers) = collected.trailers() {
                for (k, v) in trailers {
                    if let Ok(s) = v.to_str() {
                        trailer_headers.insert(k.as_str().to_owned(), s.to_owned());
                    }
                }
            }

            // Check grpc-status: prefer headers, fall back to trailers.
            let grpc_status_str = parts
                .headers
                .get("grpc-status")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_else(|| {
                    trailer_headers
                        .get("grpc-status")
                        .map(|s| s.as_str())
                        .unwrap_or("0")
                })
                .to_owned();

            if grpc_status_str != "0" {
                let code_int: i32 = grpc_status_str.parse().unwrap_or(2 /* Unknown */);
                let msg = parts
                    .headers
                    .get("grpc-message")
                    .and_then(|v| v.to_str().ok())
                    .or_else(|| trailer_headers.get("grpc-message").map(|s| s.as_str()))
                    .unwrap_or("")
                    .to_owned();
                return Err(GrpcEgressError::Status(
                    StatusConversions::from_wire(code_int),
                    msg,
                ));
            }

            let data = collected.to_bytes();
            // Strip the 5-byte gRPC frame header if present.
            let body_bytes = if data.len() >= 5 {
                data[5..].to_vec()
            } else {
                data.to_vec()
            };

            Ok(GrpcResponse {
                body: body_bytes,
                metadata: trailer_headers,
            })
        })
    }

    fn health_check(
        &self,
        _req: crate::api::HealthCheckRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<()>> {
        let pool = Arc::clone(&self.pool);
        Box::pin(async move {
            LoadbalancerSvc::select(&pool)
                .map(|_| ())
                .map_err(|e| GrpcEgressError::Unavailable(e.to_string()))
        })
    }
}

#[cfg(feature = "prost")]
impl crate::api::GrpcEgressProstCodec for TonicLbGrpcEgress {}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use swe_edge_loadbalancer::{BackendConfig, LoadbalancerConfig, Strategy};

    use super::{TonicLbGrpcEgress, DEFAULT_TIMEOUT};

    fn one_backend(url: &str) -> LoadbalancerConfig {
        LoadbalancerConfig {
            strategy: Strategy::RoundRobin,
            backends: vec![BackendConfig {
                url: url.to_string(),
                weight: 1,
            }],
        }
    }

    // ── from_config: error paths (no runtime needed) ─────────────────────────

    /// @covers: from_config
    #[test]
    fn test_from_config_empty_backends_returns_unavailable() {
        let config = LoadbalancerConfig {
            strategy: Strategy::RoundRobin,
            backends: vec![],
        };
        let err = TonicLbGrpcEgress::from_config(config).unwrap_err();
        assert!(
            err.to_string().contains("no backends"),
            "expected 'no backends' in error, got: {err}"
        );
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_invalid_url_returns_unavailable() {
        let config = one_backend("!! not a valid url !!");
        let err = TonicLbGrpcEgress::from_config(config).unwrap_err();
        assert!(
            err.to_string().contains("invalid backend URL"),
            "expected 'invalid backend URL' in error, got: {err}"
        );
    }

    // ── from_config: happy path (Channel::balance_list needs a Tokio runtime) ─

    /// @covers: from_config
    #[test]
    fn test_from_config_valid_url_builds_client() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let client = rt.block_on(async {
            TonicLbGrpcEgress::from_config(one_backend("http://localhost:50051"))
        });
        assert!(client.is_ok(), "expected Ok, got: {:?}", client.err());
    }

    // ── with_timeout / timeout ────────────────────────────────────────────────

    /// @covers: with_timeout
    #[test]
    fn test_with_timeout_overrides_default() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let client = rt.block_on(async {
            TonicLbGrpcEgress::from_config(one_backend("http://localhost:50051"))
                .unwrap()
                .with_timeout(Duration::from_secs(5))
        });
        assert_eq!(client.timeout, Duration::from_secs(5));
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_defaults_timeout_when_not_overridden() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let client = rt.block_on(async {
            TonicLbGrpcEgress::from_config(one_backend("http://localhost:50051")).unwrap()
        });
        assert_eq!(client.timeout, DEFAULT_TIMEOUT);
    }

    /// @covers: health_check
    #[tokio::test]
    async fn test_health_check_with_healthy_pool_returns_ok() {
        use crate::api::{GrpcEgress, HealthCheckRequest};
        let client = TonicLbGrpcEgress::from_config(one_backend("http://localhost:50051"))
            .expect("valid config");
        // Pool has one healthy backend — health_check only probes pool membership,
        // no network call is made.
        assert!(client.health_check(HealthCheckRequest).await.is_ok());
    }
}
