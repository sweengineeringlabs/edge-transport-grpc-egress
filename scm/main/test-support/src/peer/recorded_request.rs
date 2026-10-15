//! [`RecordedRequest`] -- what a peer saw of one request.

use std::collections::HashMap;

use crate::DecodedFrame;

/// The path, the headers and the messages of one request received by a
/// [`RunningPeer`](crate::RunningPeer).
#[derive(Debug, Clone)]
pub struct RecordedRequest {
    path: String,
    headers: HashMap<String, String>,
    frames: Vec<DecodedFrame>,
}

impl RecordedRequest {
    /// A record of a request.
    pub fn create(
        path: String,
        headers: HashMap<String, String>,
        frames: Vec<DecodedFrame>,
    ) -> Self {
        Self {
            path,
            headers,
            frames,
        }
    }

    /// The request path, for example `/pkg.Service/Method`.
    pub fn get_path(&self) -> &str {
        &self.path
    }

    /// The value of a header, by lower-case name, when the request had it.
    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }

    /// The messages of the body, in order.
    pub fn get_frames(&self) -> &[DecodedFrame] {
        &self.frames
    }
}
