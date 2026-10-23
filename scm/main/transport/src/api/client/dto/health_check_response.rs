//! Response message for `grpc.health.v1.Health/Check`.

use proto_pattern::{DecodeError, EncodeError, Payload};

use super::super::vo::HealthStatus;
use super::health_proto_codec::HealthProtoCodec;

/// The serving status of the service asked about.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HealthCheckResponse {
    /// The numeric status from the protocol; use [`Self::status`] for its known enum value.
    pub status: i32,
}

impl HealthCheckResponse {
    /// Returns the known status, mapping an unrecognized protocol value to `Unknown`.
    pub fn status(&self) -> HealthStatus {
        self.status.into()
    }
}

impl Payload for HealthCheckResponse {
    const FULL_NAME: &'static str = "grpc.health.v1.HealthCheckResponse";

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        if self.status != 0 {
            HealthProtoCodec::write_varint(out, 0x08);
            HealthProtoCodec::write_varint(out, self.status as i64 as u64);
        }
        Ok(())
    }

    fn estimate_encoded_len(&self) -> usize {
        if self.status == 0 {
            0
        } else {
            1 + HealthProtoCodec::varint_len(self.status as i64 as u64)
        }
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = HealthProtoCodec::reader(bytes, Self::FULL_NAME);
        let mut status = 0i32;
        while let Some((field, wire_type)) = reader.next_field()? {
            if field == 1 && wire_type == 0 {
                status = reader.read_varint()? as i32;
            } else {
                reader.skip(wire_type)?;
            }
        }
        Ok(Self { status })
    }
}
