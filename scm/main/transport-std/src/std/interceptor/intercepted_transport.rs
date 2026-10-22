//! `InterceptedTransport` -- a client whose calls run through a chain of interceptors.

use grpc_egress_transport::InboundReply;
use proto_pattern::{
    BidiStreamMethod, CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallOptions, CallRawBidiStreamRequest,
    CallRawBidiStreamRequestBuilder, CallRawBidiStreamResponse, CallRawClientStreamRequest,
    CallRawClientStreamRequestBuilder, CallRawClientStreamResponse, CallRawRequest,
    CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequest,
    CallRawServerStreamRequestBuilder, CallRawServerStreamResponse, CallRequest, CallResponse,
    CallServerStreamRequest, CallServerStreamResponse, ClientStreamMethod, MessageStream,
    MetadataView, RawTransport, ResponseStream, ServerStreamMethod, StatusCode, Transport,
    UnaryMethod,
};

use crate::std::client::MetadataExtender;
use crate::std::{GrpcEgressInterceptorChain, PendingCall};

/// Runs `chain` around every call of `inner`: `before_call` on every shape before anything is sent, `after_call`
/// after a unary or client-streaming response. A call with no interceptor in the chain is forwarded as it is.
///
/// A typed call is forwarded as a typed call and a raw call as a raw call, so nothing is copied or encoded
/// here.
pub(crate) struct InterceptedTransport<T> {
    inner: T,
    chain: GrpcEgressInterceptorChain,
}

impl<T> InterceptedTransport<T> {
    /// `inner` with `chain` around its calls.
    pub(crate) fn create(inner: T, chain: GrpcEgressInterceptorChain) -> Self {
        Self { inner, chain }
    }

    /// The options a call carries after every `before_call` has run.
    fn run_before(&self, path: &str, options: CallOptions) -> Result<CallOptions, CallError> {
        if self.chain.is_empty() {
            return Ok(options);
        }
        let mut call = PendingCall::create(path, &options);
        self.chain.run_before(&mut call)?;
        Ok(call.into_options())
    }

    /// Run every `after_call` on a response with a message.
    fn run_after<H: MetadataView>(
        &self,
        path: &str,
        headers: &H,
        trailers: &H,
    ) -> Result<(), CallError> {
        if self.chain.is_empty() {
            return Ok(());
        }
        self.chain.run_after(&InboundReply {
            path,
            headers: MetadataExtender::copy(headers).map_err(|error| CallError::Transport {
                path: path.into(),
                code: StatusCode::Internal,
                reason: error.to_string(),
            })?,
            trailers: MetadataExtender::copy(trailers).map_err(|error| CallError::Transport {
                path: path.into(),
                code: StatusCode::Internal,
                reason: error.to_string(),
            })?,
        })
    }
}

impl<T: Transport + RawTransport> Transport for InterceptedTransport<T> {
    type Metadata = <T as Transport>::Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        let (message, options) = request.into_parts();
        let options = self.run_before(M::PATH, options)?;
        let response = self
            .inner
            .call::<M>(CallRequest::new(message, options))
            .await?;
        self.run_after(M::PATH, response.get_headers(), response.get_trailers())?;
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

        let options = self.run_before(M::PATH, options)?;
        self.inner
            .call_server_stream::<M>(CallServerStreamRequest::new(message, options))
            .await
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        request: CallClientStreamRequest<S>,
    ) -> Result<CallClientStreamResponse<M::Response, <Self as Transport>::Metadata>, CallError>
    {
        let (requests, options) = request.into_parts();
        let options = self.run_before(M::PATH, options)?;
        let response = self
            .inner
            .call_client_stream::<M, S>(CallClientStreamRequest::new(requests, options))
            .await
            .map(CallClientStreamResponse::into_response)?;
        self.run_after(M::PATH, response.get_headers(), response.get_trailers())?;
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
        let options = self.run_before(M::PATH, options)?;
        self.inner
            .call_bidi_stream::<M, S>(CallBidiStreamRequest::new(requests, options))
            .await
    }
}

impl<T: Transport + RawTransport> RawTransport for InterceptedTransport<T> {
    type Metadata = <T as RawTransport>::Metadata;

    async fn call_raw(
        &self,
        request: CallRawRequest,
    ) -> Result<CallRawResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, message, options) = request.into_parts();

        let options = self.run_before(path.get_path(), options)?;
        let response = self
            .inner
            .call_raw(
                CallRawRequestBuilder::new()
                    .with_path(path.clone())
                    .with_message(message)
                    .with_options(options)
                    .build(),
            )
            .await
            .map(CallRawResponse::into_response)?;
        self.run_after(
            path.get_path(),
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

        let options = self.run_before(path.get_path(), options)?;
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
        let options = self.run_before(path.get_path(), options)?;
        let response = self
            .inner
            .call_raw_client_stream(
                CallRawClientStreamRequestBuilder::new()
                    .with_path(path.clone())
                    .with_requests(requests)
                    .with_options(options)
                    .build(),
            )
            .await
            .map(CallRawClientStreamResponse::into_response)?;
        self.run_after(
            path.get_path(),
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
        let options = self.run_before(path.get_path(), options)?;
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
