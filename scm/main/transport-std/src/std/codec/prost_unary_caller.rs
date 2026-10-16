//! [`ProstUnaryCaller`] -- a typed unary call: encode a prost message, call, decode the answer.

use std::time::Duration;

use grpc_egress_transport::{GrpcEgressError, GrpcEgressProstCodec, GrpcEgressResult, GrpcRequest};

use crate::std::GrpcRequestFactory;

/// Typed unary calls for any [`GrpcEgress`](grpc_egress_transport::GrpcEgress): the message types are
/// prost messages, the transport stays byte oriented. Zero-sized; never instantiated.
pub struct ProstUnaryCaller;

impl ProstUnaryCaller {
    /// Encode `req` via prost, dispatch `method` on `client` with `deadline`, then decode the
    /// response body via prost.
    ///
    /// Transport- and status-level errors from the underlying
    /// [`call_unary_encoded`](GrpcEgressProstCodec::call_unary_encoded) propagate unchanged. A
    /// response body that cannot be decoded is an unexpected client-side condition, mapped to
    /// [`GrpcEgressError::Internal`].
    pub async fn call_unary_typed<C, Req, Resp>(
        client: &C,
        method: &str,
        req: &Req,
        deadline: Duration,
    ) -> GrpcEgressResult<Resp>
    where
        C: GrpcEgressProstCodec + ?Sized,
        Req: prost::Message,
        Resp: prost::Message + Default + 'static,
    {
        let request = GrpcRequest::new(method, req.encode_to_vec(), deadline);
        let response = client.call_unary_encoded(request).await?;
        Resp::decode(response.body.as_slice())
            .map_err(|e| GrpcEgressError::Internal(format!("response decode failed: {e}")))
    }
}
