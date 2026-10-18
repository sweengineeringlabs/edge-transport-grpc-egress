//! `ContextTransport` -- a view of a client whose calls carry the headers of a caller's identity.

use grpc_egress_transport::proto_pattern::{
    BidiStreamMethod, CallError, CallOptions, CallRequest, CallResponse, ClientStreamMethod,
    MessageStream, MethodPath, RawTransport, ResponseStream, ServerStreamMethod, StatusCode,
    Transport, UnaryMethod,
};

use super::ContextHeaders;
use crate::std::client::StdMetadataExtender;
use grpc_egress_transport::{BuildGrpcCallerIdentityRequest, MetadataExtender};

/// A view of `inner` bound to an identity: every call carries the headers derived from it, except one the call
/// already carries. The headers are derived when the view is made, so a call pays one copy of its own headers.
///
/// A header that is not valid metadata (an identity entry with a key gRPC does not allow) fails the call with
/// `InvalidArgument` and nothing is sent: an identity header is never dropped silently.
pub(crate) struct ContextTransport<'a, T> {
    inner: &'a T,
    headers: Vec<(String, String)>,
}

impl<'a, T> From<BuildGrpcCallerIdentityRequest<'a, '_, T>> for ContextTransport<'a, T> {
    /// A view of the client that carries the headers of the identity.
    fn from(req: BuildGrpcCallerIdentityRequest<'a, '_, T>) -> Self {
        Self {
            inner: req.inner,
            headers: ContextHeaders::from(req.identity).entries,
        }
    }
}

impl<T> ContextTransport<'_, T> {
    /// `options` with the headers of the context added.
    fn add_headers(&self, path: &str, options: CallOptions) -> Result<CallOptions, CallError> {
        if self.headers.is_empty() {
            return Ok(options);
        }
        let metadata = StdMetadataExtender
            .extend(
                options.get_metadata(),
                self.headers.iter().map(|(k, v)| (k.as_str(), v.as_str())),
            )
            .map_err(|e| CallError::Transport {
                path: path.into(),
                code: StatusCode::InvalidArgument,
                reason: format!("the identity holds a header that is not valid: {e}"),
            })?;
        Ok(CallOptions::create(options.get_timeout(), metadata))
    }
}

impl<T: Transport + RawTransport> Transport for ContextTransport<'_, T> {
    type Metadata = <T as Transport>::Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        let (message, options) = request.into_parts();
        let options = self.add_headers(M::PATH, options)?;
        self.inner
            .call::<M>(CallRequest::create(message, options))
            .await
    }

    async fn call_server_stream<M: ServerStreamMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>, CallError>
    {
        let (message, options) = request.into_parts();
        let options = self.add_headers(M::PATH, options)?;
        self.inner
            .call_server_stream::<M>(CallRequest::create(message, options))
            .await
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        requests: S,
        options: CallOptions,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        let options = self.add_headers(M::PATH, options)?;
        self.inner
            .call_client_stream::<M, S>(requests, options)
            .await
    }

    async fn call_bidi_stream<M: BidiStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        requests: S,
        options: CallOptions,
    ) -> Result<impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>, CallError>
    {
        let options = self.add_headers(M::PATH, options)?;
        self.inner.call_bidi_stream::<M, S>(requests, options).await
    }
}

impl<T: Transport + RawTransport> RawTransport for ContextTransport<'_, T> {
    type Metadata = <T as RawTransport>::Metadata;

    async fn call_raw(
        &self,
        path: MethodPath,
        request: CallRequest<Vec<u8>>,
    ) -> Result<CallResponse<Vec<u8>, <Self as RawTransport>::Metadata>, CallError> {
        let (message, options) = request.into_parts();
        let options = self.add_headers(path.get_path(), options)?;
        self.inner
            .call_raw(path, CallRequest::create(message, options))
            .await
    }

    async fn call_raw_server_stream(
        &self,
        path: MethodPath,
        request: CallRequest<Vec<u8>>,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>, CallError>
    {
        let (message, options) = request.into_parts();
        let options = self.add_headers(path.get_path(), options)?;
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
        let options = self.add_headers(path.get_path(), options)?;
        self.inner
            .call_raw_client_stream(path, requests, options)
            .await
    }

    async fn call_raw_bidi_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        requests: S,
        options: CallOptions,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>, CallError>
    {
        let options = self.add_headers(path.get_path(), options)?;
        self.inner
            .call_raw_bidi_stream(path, requests, options)
            .await
    }
}
