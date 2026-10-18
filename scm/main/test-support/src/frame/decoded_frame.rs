//! [`DecodedFrame`] -- one gRPC message read out of a request or response body.

/// One length-prefixed gRPC message: the compressed flag of its prefix and its payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedFrame {
    compressed: bool,
    payload: Vec<u8>,
}

impl DecodedFrame {
    /// A frame with the given compressed flag and payload.
    pub fn create(compressed: bool, payload: Vec<u8>) -> Self {
        Self {
            compressed,
            payload,
        }
    }

    /// Whether the prefix marked the payload as compressed.
    pub fn is_compressed(&self) -> bool {
        self.compressed
    }

    /// The payload exactly as it was on the wire.
    pub fn get_payload(&self) -> &[u8] {
        &self.payload
    }
}
