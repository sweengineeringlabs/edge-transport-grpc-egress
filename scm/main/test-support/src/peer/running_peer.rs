//! [`RunningPeer`] -- a started peer, and what it has received.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::task::JoinHandle;

use crate::RecordedRequest;

/// A peer that is accepting connections. Dropping it stops the accept loop.
pub struct RunningPeer {
    addr: SocketAddr,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    accept_loop: JoinHandle<()>,
}

impl RunningPeer {
    /// A running peer: its address, the shared record of requests, and its accept loop.
    pub fn create(
        addr: SocketAddr,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
        accept_loop: JoinHandle<()>,
    ) -> Self {
        Self {
            addr,
            requests,
            accept_loop,
        }
    }

    /// The address the peer listens on.
    pub fn get_addr(&self) -> SocketAddr {
        self.addr
    }

    /// The `http://` endpoint of the peer.
    pub fn get_endpoint(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Every request received so far, in arrival order.
    pub fn get_recorded_requests(&self) -> Vec<RecordedRequest> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for RunningPeer {
    fn drop(&mut self) {
        self.accept_loop.abort();
    }
}
