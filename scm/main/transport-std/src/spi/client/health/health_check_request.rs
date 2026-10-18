//! `HealthCheckRequest` -- the request of `grpc.health.v1.Health/Check`.

use proto_pattern::{DecodeError, EncodeError, Payload};

/// Asks for the health of one service; an empty name asks for the server as a whole.
#[derive(Clone, PartialEq, prost::Message)]
pub struct HealthCheckRequest {
    /// The name of the service, as the server registered it; empty for the server.
    #[prost(string, tag = "1")]
    pub service: String,
}

impl Payload for HealthCheckRequest {
    const FULL_NAME: &'static str = "grpc.health.v1.HealthCheckRequest";

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        prost::Message::encode(self, out)
            .map_err(|e| EncodeError::create(Self::FULL_NAME, e.to_string()))
    }

    fn estimate_encoded_len(&self) -> usize {
        prost::Message::encoded_len(self)
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        prost::Message::decode(bytes)
            .map_err(|e| DecodeError::create(Self::FULL_NAME, e.to_string()))
    }
}
