//! [`CompressionModeProvider`] -- the `grpc-encoding` of a [`CompressionMode`].

use grpc_egress_transport::CompressionMode;

/// The wire name of a [`CompressionMode`]; the enum itself is plain data in the domain crate.
pub trait CompressionModeProvider {
    /// `grpc-encoding` header value, or `None` for identity-only.
    fn header_value(self) -> Option<&'static str>;
}

impl CompressionModeProvider for CompressionMode {
    fn header_value(self) -> Option<&'static str> {
        match self {
            CompressionMode::None => None,
            CompressionMode::Gzip => Some("gzip"),
            CompressionMode::Zstd => Some("zstd"),
        }
    }
}
