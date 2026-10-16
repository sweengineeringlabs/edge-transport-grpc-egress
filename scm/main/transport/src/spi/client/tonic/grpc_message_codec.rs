//! `GrpcMessageCodec` — compression of one gRPC message, per the `grpc-encoding` header.

use std::io::{Read as _, Write as _};

use crate::api::{CompressionMode, GrpcEgressError, GrpcEgressResult, GrpcStatusCode};

const SANITIZED_INTERNAL_MSG: &str = "internal client error";

/// Compresses and decompresses the payload of one gRPC message.
///
/// The compressed flag and the length prefix belong to the frame, not to the codec; the codec
/// maps bytes to bytes for one `grpc-encoding` value.
pub(crate) struct GrpcMessageCodec;

impl GrpcMessageCodec {
    /// Compress `payload` with `mode`; `None` for `CompressionMode::None`, which sends the
    /// message as it is.
    pub(crate) fn compress(
        mode: CompressionMode,
        payload: &[u8],
    ) -> GrpcEgressResult<Option<Vec<u8>>> {
        match mode {
            CompressionMode::None => Ok(None),
            CompressionMode::Gzip => {
                let mut encoder =
                    flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
                encoder
                    .write_all(payload)
                    .and_then(|()| encoder.finish())
                    .map(Some)
                    .map_err(Self::fail_compress)
            }
            CompressionMode::Zstd => {
                zstd::stream::encode_all(payload, zstd::DEFAULT_COMPRESSION_LEVEL)
                    .map(Some)
                    .map_err(Self::fail_compress)
            }
        }
    }

    /// Decompress `payload` that the peer announced as `encoding`, refusing to produce more
    /// than `max_bytes` (a small payload can expand without bound).
    ///
    /// # Errors
    ///
    /// - `Status(Unimplemented)` for an encoding this client does not speak;
    /// - `Status(ResourceExhausted)` when the decompressed message exceeds `max_bytes`;
    /// - `Internal` when the payload is not valid for the encoding.
    pub(crate) fn decompress(
        encoding: &str,
        payload: &[u8],
        max_bytes: usize,
    ) -> GrpcEgressResult<Vec<u8>> {
        // One byte past the cap tells "exactly at the cap" from "over it".
        let limit = u64::try_from(max_bytes)
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        let mut out = Vec::new();
        let read = match encoding {
            "gzip" => flate2::read::GzDecoder::new(payload)
                .take(limit)
                .read_to_end(&mut out),
            "zstd" => zstd::stream::read::Decoder::new(payload)
                .and_then(|decoder| decoder.take(limit).read_to_end(&mut out)),
            other => {
                tracing::warn!(
                    encoding = other,
                    "peer used a grpc-encoding this client does not speak"
                );
                return Err(GrpcEgressError::Status(
                    GrpcStatusCode::Unimplemented,
                    format!("unsupported grpc-encoding: {other}"),
                ));
            }
        };
        if let Err(e) = read {
            tracing::warn!(error = %e, encoding, "compressed gRPC message could not be decompressed");
            return Err(GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into()));
        }
        if out.len() > max_bytes {
            return Err(GrpcEgressError::Status(
                GrpcStatusCode::ResourceExhausted,
                "decompressed message exceeded max_message_bytes".into(),
            ));
        }
        Ok(out)
    }

    fn fail_compress(error: std::io::Error) -> GrpcEgressError {
        tracing::warn!(error = %error, "gRPC message could not be compressed");
        GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
    }
}
