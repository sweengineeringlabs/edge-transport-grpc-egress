//! `PayloadSource` -- a stream of request payloads as the contract's `MessageStream`.

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::{Stream, StreamExt as _};
use proto_pattern::{CallError, MessageStream, StatusCode};

use grpc_egress_transport::{GrpcEgressError, GrpcEgressResult};

/// The error a request stream broke with, kept so the call can report it as it was and not as the
/// contract's rendering of it.
pub(crate) type StreamFailure = Arc<Mutex<Option<GrpcEgressError>>>;

/// The request messages of a streaming call, read one at a time.
///
/// When the source yields an error the call is ended with a transport failure and the original error
/// is left in the [`StreamFailure`] for the caller of the call to report.
pub(crate) struct PayloadSource {
    stream: Pin<Box<dyn Stream<Item = GrpcEgressResult<Vec<u8>>> + Send>>,
    failure: StreamFailure,
}

impl PayloadSource {
    /// A source over `stream`, with the slot its failure is kept in.
    pub(crate) fn create(
        stream: Pin<Box<dyn Stream<Item = GrpcEgressResult<Vec<u8>>> + Send>>,
    ) -> (Self, StreamFailure) {
        let failure = StreamFailure::default();
        (
            Self {
                stream,
                failure: Arc::clone(&failure),
            },
            failure,
        )
    }
}

impl MessageStream<Vec<u8>> for PayloadSource {
    async fn read_message(&mut self) -> Result<Option<Vec<u8>>, CallError> {
        match self.stream.next().await {
            None => Ok(None),
            Some(Ok(message)) => Ok(Some(message)),
            Some(Err(error)) => {
                let reason = error.to_string();
                if let Ok(mut slot) = self.failure.lock() {
                    *slot = Some(error);
                }
                Err(CallError::Transport {
                    path: "".into(),
                    code: StatusCode::Aborted,
                    reason,
                })
            }
        }
    }
}
