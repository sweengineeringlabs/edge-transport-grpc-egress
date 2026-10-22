//! `EchoServer` -- the in-process peer the scenarios talk to.

use std::error::Error;

use grpc_egress_test_support::{Fixture, PeerBehaviour, PeerServer, RunningPeer};

/// An in-process gRPC peer on a port the operating system assigns. It records every request it gets.
pub struct EchoServer {
    peer: RunningPeer,
}

impl EchoServer {
    /// Start a peer that echoes the messages of every request back.
    ///
    /// # Errors
    ///
    /// The error of binding the listener.
    pub async fn start() -> Result<Self, Box<dyn Error>> {
        Self::start_with(PeerBehaviour::Echo).await
    }

    /// Start a peer that answers as `behaviour` says.
    ///
    /// # Errors
    ///
    /// The error of binding the listener.
    pub async fn start_with(behaviour: PeerBehaviour) -> Result<Self, Box<dyn Error>> {
        Fixture::ensure_tls_provider();
        Ok(Self {
            peer: PeerServer::start(behaviour).await?,
        })
    }

    /// The `http://host:port` endpoint of the peer.
    pub fn get_endpoint(&self) -> String {
        self.peer.get_endpoint()
    }

    /// How many requests the peer has received.
    pub fn count_requests(&self) -> usize {
        self.peer.get_recorded_requests().len()
    }

    /// The value of header `name` on the last request the peer received.
    pub fn get_last_header(&self, name: &str) -> Option<String> {
        self.peer
            .get_recorded_requests()
            .last()
            .and_then(|request| request.get_header(name))
            .map(str::to_string)
    }

    /// Whether the first message of the last request arrived compressed.
    pub fn is_last_message_compressed(&self) -> bool {
        self.peer
            .get_recorded_requests()
            .last()
            .and_then(|request| {
                request
                    .get_frames()
                    .first()
                    .map(|frame| frame.is_compressed())
            })
            .unwrap_or(false)
    }
}
