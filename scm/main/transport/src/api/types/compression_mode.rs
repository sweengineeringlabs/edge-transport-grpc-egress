//! Compression negotiation modes for gRPC channels.

use serde::{Deserialize, Serialize};

/// Wire-level compression scheme for gRPC.
///
/// The `header_value` impl lives in `core/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CompressionMode {
    /// No compression.
    #[default]
    None,
    /// gzip compression.
    Gzip,
    /// zstandard compression.
    Zstd,
}
