//! [`FreezableProxy`] -- forwards TCP to an upstream until frozen.

use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};

use crate::RunningProxy;

/// Starts proxies in front of an upstream peer.
pub struct FreezableProxy;

impl FreezableProxy {
    /// Start a proxy on a port the operating system assigns, forwarding to `upstream`.
    ///
    /// # Errors
    ///
    /// The error of binding the listener or reading its address.
    pub async fn start(upstream: SocketAddr) -> io::Result<RunningProxy> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let frozen = Arc::new(AtomicBool::new(false));
        let closed_by_client = Arc::new(AtomicUsize::new(0));

        let (frozen_for_loop, closed_for_loop) = (frozen.clone(), closed_by_client.clone());
        let accept_loop = tokio::spawn(async move {
            while let Ok((client, _)) = listener.accept().await {
                let (frozen, closed) = (frozen_for_loop.clone(), closed_for_loop.clone());
                tokio::spawn(async move {
                    let _ = Self::forward(client, upstream, frozen, closed).await;
                });
            }
        });
        Ok(RunningProxy::create(
            addr,
            frozen,
            closed_by_client,
            accept_loop,
        ))
    }

    async fn forward(
        client: TcpStream,
        upstream: SocketAddr,
        frozen: Arc<AtomicBool>,
        closed_by_client: Arc<AtomicUsize>,
    ) -> io::Result<()> {
        let upstream = TcpStream::connect(upstream).await?;
        let (mut client_read, mut client_write) = client.into_split();
        let (mut upstream_read, mut upstream_write) = upstream.into_split();

        let frozen_up = frozen.clone();
        let to_upstream = async move {
            let mut buf = [0u8; 8192];
            loop {
                match client_read.read(&mut buf).await {
                    Ok(0) | Err(_) => {
                        closed_by_client.fetch_add(1, Ordering::SeqCst);
                        return;
                    }
                    Ok(n) if !frozen_up.load(Ordering::SeqCst) => {
                        if upstream_write.write_all(&buf[..n]).await.is_err() {
                            return;
                        }
                    }
                    Ok(_) => {}
                }
            }
        };
        let to_client = async move {
            let mut buf = [0u8; 8192];
            loop {
                match upstream_read.read(&mut buf).await {
                    Ok(0) | Err(_) => return,
                    Ok(n) if !frozen.load(Ordering::SeqCst) => {
                        if client_write.write_all(&buf[..n]).await.is_err() {
                            return;
                        }
                    }
                    Ok(_) => {}
                }
            }
        };
        tokio::join!(to_upstream, to_client);
        Ok(())
    }
}
