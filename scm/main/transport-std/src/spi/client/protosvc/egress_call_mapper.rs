//! `EgressCallMapper` -- conversions between this crate's call types and `proto-pattern`'s.

use std::collections::HashMap;

use proto_pattern::{CallError, Metadata, MetadataEntry, MetadataView, MethodPath, StatusCode};

use crate::std::Conversions as StatusConversions;
use grpc_egress_transport::{GrpcEgressError, GrpcEgressResult, GrpcStatusCode};

/// What a caller is told when the reason is internal: the reason itself goes to the log.
const SANITIZED_INTERNAL_MSG: &str = "internal client error";

/// Trailer names that carry the time the peer asks the caller to wait, in seconds.
const RETRY_AFTER_KEYS: [&str; 2] = ["retry-after", "x-ratelimit-reset-requests"];

/// The conversions the bridge needs, each a pure function.
pub(crate) struct EgressCallMapper;

impl EgressCallMapper {
    /// The path of `method`, written with or without the leading `/`.
    ///
    /// # Errors
    ///
    /// `Internal` when the text is not a `/service/method` path: the reason is logged, not returned.
    pub(crate) fn create_path(method: &str) -> GrpcEgressResult<MethodPath> {
        MethodPath::create(format!("/{}", method.trim_start_matches('/'))).map_err(|e| {
            tracing::warn!(error = %e, method = %method, "invalid gRPC method path supplied by caller");
            GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
        })
    }

    /// The metadata of `map`.
    ///
    /// # Errors
    ///
    /// `Internal` when a key or a value is not valid metadata: the reason is logged, not returned.
    pub(crate) fn create_metadata(map: &HashMap<String, String>) -> GrpcEgressResult<Metadata> {
        let mut entries = Vec::with_capacity(map.len());
        for (key, value) in map {
            let entry = MetadataEntry::create(key.as_str(), value.as_str()).map_err(|e| {
                tracing::warn!(error = %e, key = %key, "invalid gRPC metadata supplied by caller");
                GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
            })?;
            entries.push(entry);
        }
        Ok(Metadata::create(entries))
    }

    /// The text entries of `metadata` as a map; where a key repeats the last value wins.
    pub(crate) fn read_metadata<M: MetadataView>(metadata: &M) -> HashMap<String, String> {
        metadata
            .get_entries()
            .map(|entry| (entry.get_key().to_owned(), entry.get_value().to_owned()))
            .collect()
    }

    /// This crate's status code for `code`.
    pub(crate) fn adapt_code(code: StatusCode) -> GrpcStatusCode {
        StatusConversions::from_wire(code.get_code())
    }

    /// The error this crate reports for the contract's `error`.
    ///
    /// The peer's own status and message pass through as they are; every other failure is reduced to
    /// the sanitized message of its kind and the reason is logged.
    pub(crate) fn adapt_error(error: CallError) -> GrpcEgressError {
        match error {
            CallError::Status {
                code,
                message,
                trailers,
                ..
            } => {
                let code = Self::adapt_code(code);
                let message = match (code, Self::read_retry_after(&trailers)) {
                    (GrpcStatusCode::ResourceExhausted, Some(secs)) => {
                        format!("{message} [retry-after={secs}s]")
                    }
                    _ => message,
                };
                GrpcEgressError::Status(code, message)
            }
            CallError::Transport { code, reason, .. } => {
                tracing::warn!(code = %code, reason = %reason, "gRPC transport failure");
                match code {
                    StatusCode::Unavailable | StatusCode::Unknown => {
                        GrpcEgressError::ConnectionFailed("transport error".into())
                    }
                    StatusCode::DeadlineExceeded => GrpcEgressError::Status(
                        GrpcStatusCode::DeadlineExceeded,
                        "the call did not complete within its timeout".into(),
                    ),
                    StatusCode::Cancelled => {
                        GrpcEgressError::Cancelled("the call was cancelled".into())
                    }
                    StatusCode::ResourceExhausted => GrpcEgressError::Status(
                        GrpcStatusCode::ResourceExhausted,
                        "the call exceeded a size limit".into(),
                    ),
                    _ => GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into()),
                }
            }
            CallError::Encode(source) => {
                tracing::warn!(error = %source, "gRPC request could not be encoded");
                if source.is_too_large() {
                    GrpcEgressError::Status(
                        GrpcStatusCode::ResourceExhausted,
                        "request exceeded max_message_bytes".into(),
                    )
                } else {
                    GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
                }
            }
            CallError::Decode(source) => {
                tracing::warn!(error = %source, "gRPC response could not be decoded");
                GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
            }
        }
    }

    /// The seconds the peer asked the caller to wait, from the first trailer that says so.
    fn read_retry_after(trailers: &Metadata) -> Option<u64> {
        RETRY_AFTER_KEYS.iter().find_map(|key| {
            trailers
                .get_values(key)
                .first()
                .and_then(|value| value.parse::<u64>().ok())
        })
    }
}
