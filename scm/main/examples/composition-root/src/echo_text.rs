//! `EchoText` -- a typed method the example calls: a text message, answered with the same text.

use proto_pattern::{DecodeError, EncodeError, Method, Payload, UnaryMethod};

/// A text message that is its own encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text(pub String);

impl Payload for Text {
    const FULL_NAME: &'static str = "example.Text";

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        out.extend_from_slice(self.0.as_bytes());
        Ok(())
    }

    fn estimate_encoded_len(&self) -> usize {
        self.0.len()
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        String::from_utf8(bytes.to_vec())
            .map(Text)
            .map_err(|e| DecodeError::create(Self::FULL_NAME, e.to_string()))
    }
}

/// `/pkg.Service/Method` as a typed unary method.
pub struct EchoText;

impl Method for EchoText {
    type Request = Text;
    type Response = Text;
    const PATH: &'static str = "/pkg.Service/Method";
}

impl UnaryMethod for EchoText {}
