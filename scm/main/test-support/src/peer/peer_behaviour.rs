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
}
