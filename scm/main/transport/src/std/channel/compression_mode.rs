//! `impl` block for [`CompressionMode`]. The type *declaration* lives in `api/`.

use crate::api::CompressionMode;

impl CompressionMode {
    /// `grpc-encoding` header value, or `None` for identity-only.
    pub fn header_value(self) -> Option<&'static str> {
        match self {
            CompressionMode::None => None,
            CompressionMode::Gzip => Some("gzip"),
            CompressionMode::Zstd => Some("zstd"),
        }
    }
}
