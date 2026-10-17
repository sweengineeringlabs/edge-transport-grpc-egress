//! `InterceptedTransport` -- a client whose calls run through a chain of interceptors.

use grpc_egress_transport::InboundReply;
use proto_pattern::{
    BidiStreamMethod, CallError, CallOptions, CallRequest, CallResponse, ClientStreamMethod,
    MessageStream, MetadataView, MethodPath, RawTransport, ResponseStream, ServerStreamMethod,
    Transport, UnaryMethod,
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
            headers: MetadataExtender::copy(headers),
            trailers: MetadataExtender::copy(trailers),
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
            .call::<M>(CallRequest::create(message, options))
            .await?;
        self.run_after(M::PATH, response.get_headers(), response.get_trailers())?;
        Ok(response)
    }

    async fn call_server_stream<M: ServerStreamMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>, CallError>
    {
        let (message, options) = request.into_parts();
        let options = self.run_before(M::PATH, options)?;
        self.inner
            .call_server_stream::<M>(CallRequest::create(message, options))
            .await
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        requests: S,
        options: CallOptions,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        let options = self.run_before(M::PATH, options)?;
        let response = self
            .inner
            .call_client_stream::<M, S>(requests, options)
            .await?;
        self.run_after(M::PATH, response.get_headers(), response.get_trailers())?;
        Ok(response)
    }

    async fn call_bidi_stream<M: BidiStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        requests: S,
        options: CallOptions,
    ) -> Result<impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>, CallError>
    {
        let options = self.run_before(M::PATH, options)?;
        self.inner.call_bidi_stream::<M, S>(requests, options).await
    }
}

impl<T: Transport + RawTransport> RawTransport for InterceptedTransport<T> {
    type Metadata = <T as RawTransport>::Metadata;

    async fn call_raw(
        &self,
        path: MethodPath,
        request: CallRequest<Vec<u8>>,
    ) -> Result<CallResponse<Vec<u8>, <Self as RawTransport>::Metadata>, CallError> {
        let (message, options) = request.into_parts();
        let options = self.run_before(path.get_path(), options)?;
        let response = self
            .inner
            .call_raw(path.clone(), CallRequest::create(message, options))
            .await?;
        self.run_after(
            path.get_path(),
            response.get_headers(),
            response.get_trailers(),
        )?;
        Ok(response)
    }

    async fn call_raw_server_stream(
        &self,
        path: MethodPath,
        request: CallRequest<Vec<u8>>,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>, CallError>
    {
        let (message, options) = request.into_parts();
        let options = self.run_before(path.get_path(), options)?;
        self.inner
            .call_raw_server_stream(path, CallRequest::create(message, options))
            .await
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        requests: S,
        options: CallOptions,
    ) -> Result<CallResponse<Vec<u8>, <Self as RawTransport>::Metadata>, CallError> {
        let options = self.run_before(path.get_path(), options)?;
        let response = self
            .inner
            .call_raw_client_stream(path.clone(), requests, options)
            .await?;
        self.run_after(
            path.get_path(),
            response.get_headers(),
            response.get_trailers(),
        )?;
        Ok(response)
    }

    async fn call_raw_bidi_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        requests: S,
        options: CallOptions,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>, CallError>
    {
        let options = self.run_before(path.get_path(), options)?;
        self.inner
            .call_raw_bidi_stream(path, requests, options)
            .await
    }
}
