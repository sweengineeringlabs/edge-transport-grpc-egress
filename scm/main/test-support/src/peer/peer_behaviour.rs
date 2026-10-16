//! [`PeerBehaviour`] -- what an in-process peer answers.

use std::time::Duration;

/// How a [`PeerServer`](crate::PeerServer) answers every request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerBehaviour {
    /// Answer with the same messages the request carried.
    Echo,
    /// Answer with one message holding exactly these bytes, whatever the request was.
    Reply(Vec<u8>),
    /// Wait this long before answering with the request's messages.
    Delay(Duration),
    /// Answer with one message marked compressed whose wire bytes are exactly `payload`,
    /// announcing `encoding` in `grpc-encoding` (or announcing nothing when it is `None`).
    ReplyCompressed {
        /// The `grpc-encoding` header value, if any.
        encoding: Option<String>,
        /// The bytes of the message as they travel, already compressed (or not valid at all).
        payload: Vec<u8>,
    },
    /// Answer with no message and the `grpc-status` header `status`, written as given so that a value
    /// that is no integer can be sent. `message` goes into `grpc-message`, and `retry_after_secs`
    /// into `retry-after`.
    ReplyStatus {
        /// The `grpc-status` header value.
        status: String,
        /// The `grpc-message` header value, if any.
        message: Option<String>,
        /// The `retry-after` header value in seconds, if any.
        retry_after_secs: Option<u64>,
    },
    /// Answer with exactly these body bytes, framed or not, announcing `encoding` in `grpc-encoding`
    /// when it is given. For responses that are not valid gRPC frames.
    ReplyRaw {
        /// The response body as it travels.
        body: Vec<u8>,
        /// The `grpc-encoding` header value, if any.
        encoding: Option<String>,
    },
}
