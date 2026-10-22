//! [`RunningProxy`] -- a started proxy: freeze it, and see what its clients did.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

/// A proxy that is accepting connections. Dropping it stops the accept loop.
pub struct RunningProxy {
    addr: SocketAddr,
    frozen: Arc<AtomicBool>,
    closed_by_client: Arc<AtomicUsize>,
    accept_loop: JoinHandle<()>,
}

impl RunningProxy {
    /// A running proxy: its address, the shared state its connections use, and its accept loop.
    pub fn create(
        addr: SocketAddr,
        frozen: Arc<AtomicBool>,
        closed_by_client: Arc<AtomicUsize>,
        accept_loop: JoinHandle<()>,
    ) -> Self {
        Self {
            addr,
            frozen,
            closed_by_client,
            accept_loop,
        }
    }

    /// The address the proxy listens on.
    pub fn get_addr(&self) -> SocketAddr {
        self.addr
    }

    /// The `http://` endpoint of the proxy.
    pub fn get_endpoint(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// From now on, forward nothing in either direction. Connections stay open, so the peer
    /// behind the proxy looks alive but never answers again, not even to an HTTP/2 PING.
    pub fn freeze(&self) {
        self.frozen.store(true, Ordering::SeqCst);
    }

    /// How many client connections the client has closed so far.
    pub fn get_closed_by_client(&self) -> usize {
        self.closed_by_client.load(Ordering::SeqCst)
    }

    /// Wait until a client has closed a connection, for at most `limit`; whether one did.
    pub async fn wait_for_client_close(&self, limit: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + limit;
        while tokio::time::Instant::now() < deadline {
            if self.get_closed_by_client() > 0 {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        self.get_closed_by_client() > 0
    }
}

impl Drop for RunningProxy {
    fn drop(&mut self) {
        self.accept_loop.abort();
    }
}
