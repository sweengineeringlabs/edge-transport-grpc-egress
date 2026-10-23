//! Request message for `grpc.health.v1.Health/Check`.

use proto_pattern::{DecodeError, EncodeError, Payload};

use super::health_proto_codec::HealthProtoCodec;

/// Asks for the health of one service; an empty name asks for the server as a whole.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HealthCheckRequest {
    /// The service name registered on the server, or empty for the whole server.
    pub service: String,
}

impl Payload for HealthCheckRequest {
    const FULL_NAME: &'static str = "grpc.health.v1.HealthCheckRequest";

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        if !self.service.is_empty() {
            HealthProtoCodec::write_varint(out, 0x0a);
            HealthProtoCodec::write_varint(out, self.service.len() as u64);
            out.extend_from_slice(self.service.as_bytes());
        }
        Ok(())
    }

    fn estimate_encoded_len(&self) -> usize {
        if self.service.is_empty() {
            0
        } else {
            1 + HealthProtoCodec::varint_len(self.service.len() as u64) + self.service.len()
        }
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = HealthProtoCodec::reader(bytes, Self::FULL_NAME);
        let mut service = String::new();
        while let Some((field, wire_type)) = reader.next_field()? {
            if field == 1 && wire_type == 2 {
                let value = reader.read_length_delimited()?;
                service = std::str::from_utf8(value)
                    .map_err(|_| {
                        DecodeError::create(Self::FULL_NAME, "service is not valid UTF-8")
                    })?
                    .to_owned();
            } else {
                reader.skip(wire_type)?;
            }
        }
        Ok(Self { service })
    }
}
