//! `ContextTransport` -- a view of a client whose calls carry the headers of a caller's identity.

use grpc_egress_transport::proto_pattern::{
    BidiStreamMethod, CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallOptions, CallRawBidiStreamRequest,
    CallRawBidiStreamRequestBuilder, CallRawBidiStreamResponse, CallRawClientStreamRequest,
    CallRawClientStreamRequestBuilder, CallRawClientStreamResponse, CallRawRequest,
    CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequest,
    CallRawServerStreamRequestBuilder, CallRawServerStreamResponse, CallRequest, CallResponse,
    CallServerStreamRequest, CallServerStreamResponse, ClientStreamMethod, MessageStream,
    RawTransport, ResponseStream, ServerStreamMethod, StatusCode, Transport, UnaryMethod,
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
            .call::<M>(CallRequest::new(message, options))
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
        let (message, options) = request.into_parts();
        let options = self.add_headers(M::PATH, options)?;
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
        let options = self.add_headers(M::PATH, options)?;
        self.inner
            .call_client_stream::<M, S>(CallClientStreamRequest::new(requests, options))
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
        let (requests, options) = request.into_parts();
        let options = self.add_headers(M::PATH, options)?;
        self.inner
            .call_bidi_stream::<M, S>(CallBidiStreamRequest::new(requests, options))
            .await
    }
}

impl<T: Transport + RawTransport> RawTransport for ContextTransport<'_, T> {
    type Metadata = <T as RawTransport>::Metadata;

    async fn call_raw(
        &self,
        request: CallRawRequest,
    ) -> Result<CallRawResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, message, options) = request.into_parts();
        let options = self.add_headers(path.get_path(), options)?;
        self.inner
            .call_raw(
                CallRawRequestBuilder::new()
                    .with_path(path)
                    .with_message(message)
                    .with_options(options)
                    .build(),
            )
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
        let options = self.add_headers(path.get_path(), options)?;
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
        let options = self.add_headers(path.get_path(), options)?;
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
        let options = self.add_headers(path.get_path(), options)?;
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
