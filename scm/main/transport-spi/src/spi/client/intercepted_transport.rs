//! `InterceptedTransport` -- a client whose calls run through a chain of interceptors.

use std::sync::Arc;

use crate::spi::client::{PendingCall, StdMetadataExtender};
use grpc_egress_transport_api::proto_pattern::{
    BidiStreamMethod, CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallEvent, CallIdentity, CallKind, CallOptions,
    CallRawBidiStreamRequest, CallRawBidiStreamRequestBuilder, CallRawBidiStreamResponse,
    CallRawClientStreamRequest, CallRawClientStreamRequestBuilder, CallRawClientStreamResponse,
    CallRawRequest, CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequest,
    CallRawServerStreamRequestBuilder, CallRawServerStreamResponse, CallRequest, CallResponse,
    CallServerStreamRequest, CallServerStreamResponse, CallStart, ClientStreamMethod,
    GetHeadersRequest, GetHeadersResponse, MessageStream, MetadataView, RawTransport,
    ReadMessageRequest, ReadMessageResponse, ReadTrailersRequest, ReadTrailersResponse,
    ResponseStream, ServerStreamMethod, StatusCode, Transport, TransportInterceptor, UnaryMethod,
};
use grpc_egress_transport_api::{GrpcEgressInterceptorChain, InboundReply, MetadataExtender};

/// Runs `chain` around every call of `inner`: `before_call` on every shape before anything is sent, `after_call`
/// after a unary or client-streaming response. A call with no interceptor in the chain is forwarded as it is.
///
/// A typed call is forwarded as a typed call and a raw call as a raw call, so nothing is copied or encoded
/// here.
pub(crate) struct InterceptedTransport<T, C> {
    inner: T,
    chain: Arc<C>,
}

impl<T, C> From<(T, C)> for InterceptedTransport<T, C> {
    fn from((inner, chain): (T, C)) -> Self {
        Self {
            inner,
            chain: Arc::new(chain),
        }
    }
}

impl<T, C: GrpcEgressInterceptorChain + Send + Sync> InterceptedTransport<T, C> {
    /// The options a call carries after every `before_call` has run.
    fn run_before(
        &self,
        adapter: &PatternEgressInterceptor<C>,
        path: &str,
        kind: CallKind,
        options: CallOptions,
    ) -> Result<CallOptions, CallError> {
        let identity = CallIdentity::new(path, kind);
        let options = adapter.before_call(CallStart::new(identity, options))?;
        adapter.on_event::<grpc_egress_transport_api::proto_pattern::Metadata>(
            CallEvent::Started { call: identity },
        );
        Ok(options)
    }

    /// Preserve egress's fallible reply policy, then report Pattern's single terminal event.
    fn run_after<H: MetadataView>(
        &self,
        adapter: &PatternEgressInterceptor<C>,
        path: &str,
        kind: CallKind,
        headers: &H,
        trailers: &H,
    ) -> Result<(), CallError> {
        if let Err(error) = self.run_after_egress(path, headers, trailers) {
            adapter.on_event::<grpc_egress_transport_api::proto_pattern::Metadata>(
                CallEvent::Failed {
                    call: CallIdentity::new(path, kind),
                    error: &error,
                },
            );
            return Err(error);
        }
        adapter.on_event::<H>(CallEvent::Completed {
            call: CallIdentity::new(path, kind),
            headers: Some(headers),
            trailers,
        });
        Ok(())
    }

    fn run_after_egress<H: MetadataView>(
        &self,
        path: &str,
        headers: &H,
        trailers: &H,
    ) -> Result<(), CallError> {
        if self.chain.is_empty() {
            return Ok(());
        }
        let headers = StdMetadataExtender
            .copy(headers)
            .map_err(|error| CallError::Transport {
                path: path.into(),
                code: StatusCode::Internal,
                reason: error.to_string(),
            })?;
        let trailers =
            StdMetadataExtender
                .copy(trailers)
                .map_err(|error| CallError::Transport {
                    path: path.into(),
                    code: StatusCode::Internal,
                    reason: error.to_string(),
                })?;
        self.chain
            .run_after(&InboundReply::new(path, headers, trailers))
    }

    fn on_error(
        &self,
        adapter: &PatternEgressInterceptor<C>,
        path: &str,
        kind: CallKind,
        error: &CallError,
    ) {
        adapter.on_event::<grpc_egress_transport_api::proto_pattern::Metadata>(CallEvent::Failed {
            call: CallIdentity::new(path, kind),
            error,
        });
    }
}

impl<T: Transport + RawTransport, C: GrpcEgressInterceptorChain + Send + Sync> Transport
    for InterceptedTransport<T, C>
{
    type Metadata = <T as Transport>::Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        let (message, options) = request.into_parts();
        let adapter = PatternEgressInterceptor::new(self.chain.clone());
        let options = self.run_before(&adapter, M::PATH, CallKind::Unary, options)?;
        let response = match self
            .inner
            .call::<M>(CallRequest::new(message, options))
            .await
        {
            Ok(response) => response,
            Err(error) => {
                self.on_error(&adapter, M::PATH, CallKind::Unary, &error);
                return Err(error);
            }
        };
        self.run_after(
            &adapter,
            M::PATH,
            CallKind::Unary,
            response.get_headers(),
            response.get_trailers(),
        )?;
        Ok(response)
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
        let (message, options) = request.into_parts();
        let adapter = PatternEgressInterceptor::new(self.chain.clone());
        let options = self.run_before(&adapter, M::PATH, CallKind::ServerStream, options)?;
        let response = match self
            .inner
            .call_server_stream::<M>(CallServerStreamRequest::new(message, options))
            .await
        {
            Ok(response) => response,
            Err(error) => {
                self.on_error(&adapter, M::PATH, CallKind::ServerStream, &error);
                return Err(error);
            }
        };
        let mut stream = PatternResponseStream {
            inner: response.into_stream(),
            adapter,
            path: M::PATH.to_owned(),
            kind: CallKind::ServerStream,
            opened: std::sync::atomic::AtomicBool::new(false),
            terminal: false,
        };
        stream.notify_opened::<M::Response>();
        Ok(CallServerStreamResponse::new(stream))
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        request: CallClientStreamRequest<S>,
    ) -> Result<CallClientStreamResponse<M::Response, <Self as Transport>::Metadata>, CallError>
    {
        let (requests, options) = request.into_parts();
        let adapter = PatternEgressInterceptor::new(self.chain.clone());
        let options = self.run_before(&adapter, M::PATH, CallKind::ClientStream, options)?;
        let response = match self
            .inner
            .call_client_stream::<M, S>(CallClientStreamRequest::new(requests, options))
            .await
            .map(CallClientStreamResponse::into_response)
        {
            Ok(response) => response,
            Err(error) => {
                self.on_error(&adapter, M::PATH, CallKind::ClientStream, &error);
                return Err(error);
            }
        };
        self.run_after(
            &adapter,
            M::PATH,
            CallKind::ClientStream,
            response.get_headers(),
            response.get_trailers(),
        )?;
        Ok(CallClientStreamResponse::new(response))
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
        let (requests, options) = request.into_parts();
        let adapter = PatternEgressInterceptor::new(self.chain.clone());
        let options = self.run_before(&adapter, M::PATH, CallKind::BidiStream, options)?;
        let response = match self
            .inner
            .call_bidi_stream::<M, S>(CallBidiStreamRequest::new(requests, options))
            .await
        {
            Ok(response) => response,
            Err(error) => {
                self.on_error(&adapter, M::PATH, CallKind::BidiStream, &error);
                return Err(error);
            }
        };
        let mut stream = PatternResponseStream {
            inner: response.into_stream(),
            adapter,
            path: M::PATH.to_owned(),
            kind: CallKind::BidiStream,
            opened: std::sync::atomic::AtomicBool::new(false),
            terminal: false,
        };
        stream.notify_opened::<M::Response>();
        Ok(CallBidiStreamResponse::new(stream))
    }
}

impl<T: Transport + RawTransport, C: GrpcEgressInterceptorChain + Send + Sync> RawTransport
    for InterceptedTransport<T, C>
{
    type Metadata = <T as RawTransport>::Metadata;

    async fn call_raw(
        &self,
        request: CallRawRequest,
    ) -> Result<CallRawResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, message, options) = request.into_parts();
        let path_str = path.get_path().to_owned();
        let adapter = PatternEgressInterceptor::new(self.chain.clone());
        let options = self.run_before(&adapter, &path_str, CallKind::Unary, options)?;
        let response = match self
            .inner
            .call_raw(
                CallRawRequestBuilder::new()
                    .with_path(path.clone())
                    .with_message(message)
                    .with_options(options)
                    .build(),
            )
            .await
            .map(CallRawResponse::into_response)
        {
            Ok(response) => response,
            Err(error) => {
                self.on_error(&adapter, &path_str, CallKind::Unary, &error);
                return Err(error);
            }
        };
        self.run_after(
            &adapter,
            &path_str,
            CallKind::Unary,
            response.get_headers(),
            response.get_trailers(),
        )?;
        Ok(CallRawResponse::new(response))
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
        let path_str = path.get_path().to_owned();
        let adapter = PatternEgressInterceptor::new(self.chain.clone());
        let options = self.run_before(&adapter, &path_str, CallKind::ServerStream, options)?;
        let response = match self
            .inner
            .call_raw_server_stream(
                CallRawServerStreamRequestBuilder::new()
                    .with_path(path)
                    .with_message(message)
                    .with_options(options)
                    .build(),
            )
            .await
        {
            Ok(response) => response,
            Err(error) => {
                self.on_error(&adapter, &path_str, CallKind::ServerStream, &error);
                return Err(error);
            }
        };
        let mut stream = PatternResponseStream {
            inner: response.into_stream(),
            adapter,
            path: path_str,
            kind: CallKind::ServerStream,
            opened: std::sync::atomic::AtomicBool::new(false),
            terminal: false,
        };
        stream.notify_opened::<Vec<u8>>();
        Ok(CallRawServerStreamResponse::new(stream))
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawClientStreamRequest<S>,
    ) -> Result<CallRawClientStreamResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, requests, options) = request.into_parts();
        let path_str = path.get_path().to_owned();
        let adapter = PatternEgressInterceptor::new(self.chain.clone());
        let options = self.run_before(&adapter, &path_str, CallKind::ClientStream, options)?;
        let response = match self
            .inner
            .call_raw_client_stream(
                CallRawClientStreamRequestBuilder::new()
                    .with_path(path.clone())
                    .with_requests(requests)
                    .with_options(options)
                    .build(),
            )
            .await
            .map(CallRawClientStreamResponse::into_response)
        {
            Ok(response) => response,
            Err(error) => {
                self.on_error(&adapter, &path_str, CallKind::ClientStream, &error);
                return Err(error);
            }
        };
        self.run_after(
            &adapter,
            &path_str,
            CallKind::ClientStream,
            response.get_headers(),
            response.get_trailers(),
        )?;
        Ok(CallRawClientStreamResponse::new(response))
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
        let path_str = path.get_path().to_owned();
        let adapter = PatternEgressInterceptor::new(self.chain.clone());
        let options = self.run_before(&adapter, &path_str, CallKind::BidiStream, options)?;
        let response = match self
            .inner
            .call_raw_bidi_stream(
                CallRawBidiStreamRequestBuilder::new()
                    .with_path(path)
                    .with_requests(requests)
                    .with_options(options)
                    .build(),
            )
            .await
        {
            Ok(response) => response,
            Err(error) => {
                self.on_error(&adapter, &path_str, CallKind::BidiStream, &error);
                return Err(error);
            }
        };
        let mut stream = PatternResponseStream {
            inner: response.into_stream(),
            adapter,
            path: path_str,
            kind: CallKind::BidiStream,
            opened: std::sync::atomic::AtomicBool::new(false),
            terminal: false,
        };
        stream.notify_opened::<Vec<u8>>();
        Ok(CallRawBidiStreamResponse::new(stream))
    }
}

include!("pattern_egress_interceptor.rs");
