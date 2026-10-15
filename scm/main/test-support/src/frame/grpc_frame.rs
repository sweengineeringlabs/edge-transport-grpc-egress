//! [`GrpcFrame`] -- the five-byte message framing of gRPC.

use bytes::{BufMut, Bytes, BytesMut};

use crate::DecodedFrame;

/// Length of the prefix: one compressed-flag byte and a four-byte big-endian length.
const PREFIX_LEN: usize = 5;

/// Encodes and decodes the length-prefixed messages of a gRPC body.
pub struct GrpcFrame;

impl GrpcFrame {
    /// One uncompressed message.
    pub fn encode(payload: &[u8]) -> Bytes {
        Self::frame(0x00, payload)
    }

    /// One message whose payload is already compressed: the flag byte is set.
    pub fn encode_compressed(payload: &[u8]) -> Bytes {
        Self::frame(0x01, payload)
    }

    /// Every complete message in `data`, in order. A trailing partial message is ignored.
    pub fn decode_all(data: &[u8]) -> Vec<DecodedFrame> {
        let mut frames = Vec::new();
        let mut rest = data;
        while rest.len() >= PREFIX_LEN {
            let length = u32::from_be_bytes([rest[1], rest[2], rest[3], rest[4]]) as usize;
            let end = PREFIX_LEN + length;
            if rest.len() < end {
                break;
            }
            frames.push(DecodedFrame::create(
                rest[0] == 0x01,
                rest[PREFIX_LEN..end].to_vec(),
            ));
            rest = &rest[end..];
        }
        frames
    }

    fn frame(flag: u8, payload: &[u8]) -> Bytes {
        let mut buffer = BytesMut::with_capacity(PREFIX_LEN + payload.len());
        buffer.put_u8(flag);
        buffer.put_u32(payload.len() as u32);
        buffer.put_slice(payload);
        buffer.freeze()
    }
}
