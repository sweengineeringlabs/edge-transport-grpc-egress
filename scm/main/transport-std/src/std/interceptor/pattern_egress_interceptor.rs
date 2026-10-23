// Adapts the egress runtime interceptor chain to Pattern's typed/raw lifecycle contract.

/// A per-call Pattern interceptor backed by the egress runtime chain.
///
/// Pattern owns the transport-neutral pre-call and event lifecycle contract. Egress keeps this
/// adapter because its interceptors are dynamically registered and may reject completed replies.
pub(crate) struct PatternEgressInterceptor {
    chain: StdGrpcEgressInterceptorChain,
}

impl PatternEgressInterceptor {
    fn new(chain: StdGrpcEgressInterceptorChain) -> Self {
        Self { chain }
    }

    fn failed(&self, path: &str, kind: CallKind, error: &CallError) {
        self.on_event::<grpc_egress_transport::proto_pattern::Metadata>(CallEvent::Failed {
            call: CallIdentity::new(path, kind),
            error,
        });
    }

}

impl TransportInterceptor for PatternEgressInterceptor {
    fn before_call(&self, call: CallStart<'_>) -> Result<CallOptions, CallError> {
        if self.chain.is_empty() {
            return Ok(call.into_options());
        }
        let path = call.get_identity().get_path();
        let options = call.into_options();
        let mut pending = PendingCall::from((path, &options));
        self.chain.run_before(&mut pending)?;
        Ok(CallOptions::from(pending))
    }

    fn on_event<M: MetadataView>(&self, _event: CallEvent<'_, M>) {
        // The egress reply policy is explicitly fallible, unlike Pattern's observational events.
        // Pattern's generic metadata callback is adapted to the concrete metadata used by tonic.
        match _event {
            CallEvent::Started { call } => self.chain.run_event(&CallEvent::Started { call }),
            CallEvent::Opened { call, headers } => {
                if let Ok(headers) = StdMetadataExtender.copy(headers) {
                    self.chain.run_event(&CallEvent::Opened {
                        call,
                        headers: &headers,
                    });
                }
            }
            CallEvent::MessageRead { call } => {
                self.chain.run_event(&CallEvent::MessageRead { call })
            }
            CallEvent::Completed {
                call,
                headers,
                trailers,
            } => {
                if let Ok(trailers) = StdMetadataExtender.copy(trailers) {
                    if let Some(headers) = headers {
                        if let Ok(headers) = StdMetadataExtender.copy(headers) {
                            self.chain.run_event(&CallEvent::Completed {
                                call,
                                headers: Some(&headers),
                                trailers: &trailers,
                            });
                        }
                    } else {
                        self.chain.run_event(&CallEvent::Completed {
                            call,
                            headers: None,
                            trailers: &trailers,
                        });
                    }
                }
            }
            CallEvent::Failed { call, error } => {
                self.chain.run_event(&CallEvent::Failed { call, error })
            }
            CallEvent::Cancelled { call } => self.chain.run_event(&CallEvent::Cancelled { call }),
        }
    }
}

/// Emits Pattern's streaming lifecycle events around a response stream.
pub(crate) struct PatternResponseStream<S> {
    inner: S,
    adapter: PatternEgressInterceptor,
    path: String,
    kind: CallKind,
    opened: std::sync::atomic::AtomicBool,
    terminal: bool,
}

impl<S> PatternResponseStream<S> {
    fn identity(&self) -> CallIdentity<'_> {
        CallIdentity::new(&self.path, self.kind)
    }

    fn notify_opened<T>(&mut self)
    where
        S: ResponseStream<T>,
    {
        // Header observation is best-effort: it must not change whether the opened stream is
        // returned to its caller. A later successful `get_headers` can still report Opened.
        if let Ok(headers) = self.inner.get_headers(GetHeadersRequest::new()) {
            let metadata = headers.into_headers();
            self.opened
                .store(true, std::sync::atomic::Ordering::Release);
            self.adapter.on_event::<S::Metadata>(CallEvent::Opened {
                call: self.identity(),
                headers: metadata,
            });
        }
    }
}

impl<S, T> MessageStream<T> for PatternResponseStream<S>
where
    S: ResponseStream<T>,
{
    async fn read_message(
        &mut self,
        request: ReadMessageRequest,
    ) -> Result<ReadMessageResponse<T>, CallError> {
        match self.inner.read_message(request).await {
            Ok(response) => {
                let message = response.into_message();
                if message.is_some() {
                    self.adapter
                        .on_event::<S::Metadata>(CallEvent::MessageRead {
                            call: self.identity(),
                        });
                }
                Ok(ReadMessageResponse::new(message))
            }
            Err(error) => {
                self.terminal = true;
                self.adapter.failed(&self.path, self.kind, &error);
                Err(error)
            }
        }
    }
}

impl<S, T> ResponseStream<T> for PatternResponseStream<S>
where
    S: ResponseStream<T>,
{
    type Metadata = S::Metadata;

    fn get_headers(
        &self,
        request: GetHeadersRequest,
    ) -> Result<GetHeadersResponse<'_, Self::Metadata>, CallError> {
        match self.inner.get_headers(request) {
            Ok(headers) => {
                let metadata = headers.into_headers();
                if !self.opened.swap(true, std::sync::atomic::Ordering::AcqRel) {
                    self.adapter.on_event::<S::Metadata>(CallEvent::Opened {
                        call: self.identity(),
                        headers: metadata,
                    });
                }
                Ok(GetHeadersResponse::new(metadata))
            }
            Err(error) => Err(error),
        }
    }

    async fn read_trailers(
        &mut self,
        request: ReadTrailersRequest,
    ) -> Result<ReadTrailersResponse<Self::Metadata>, CallError> {
        match self.inner.read_trailers(request).await {
            Ok(response) => {
                let trailers = response.into_trailers();
                self.adapter.on_event::<S::Metadata>(CallEvent::Completed {
                    call: self.identity(),
                    headers: None,
                    trailers: &trailers,
                });
                self.terminal = true;
                Ok(ReadTrailersResponse::new(trailers))
            }
            Err(error) => {
                self.terminal = true;
                self.adapter.failed(&self.path, self.kind, &error);
                Err(error)
            }
        }
    }
}

impl<S> Drop for PatternResponseStream<S> {
    fn drop(&mut self) {
        if !self.terminal {
            self.adapter.on_event::<grpc_egress_transport::proto_pattern::Metadata>(CallEvent::Cancelled {
                call: self.identity(),
            });
        }
    }
}
