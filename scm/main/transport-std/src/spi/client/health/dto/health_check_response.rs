//! `HealthCheckResponse` -- the response of `grpc.health.v1.Health/Check`.

use proto_pattern::{DecodeError, EncodeError, Payload};

use crate::spi::client::health::vo::HealthStatus;

/// The serving status of the service asked about.
#[derive(Clone, PartialEq, prost::Message)]
pub struct HealthCheckResponse {
    /// The status, as the number the protocol carries; `status()` reads it as a [`HealthStatus`].
    #[prost(enumeration = "HealthStatus", tag = "1")]
    pub status: i32,
}

impl Payload for HealthCheckResponse {
    const FULL_NAME: &'static str = "grpc.health.v1.HealthCheckResponse";

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
