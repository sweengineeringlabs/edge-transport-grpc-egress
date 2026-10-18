//! [`Fixture`] -- a reserved closed port and the TLS provider setup shared by the tests.

use std::io;
use std::sync::Once;

use crate::ClosedPort;

/// Fixtures that do not need a running peer.
pub struct Fixture;

impl Fixture {
    /// A port on `127.0.0.1` that refuses connections and stays reserved while the returned value
    /// is alive; keep it for as long as the test uses its endpoint (see [`ClosedPort`]).
    ///
    /// # Errors
    ///
    /// The error of [`ClosedPort::reserve`].
    pub fn reserve_closed_port() -> io::Result<ClosedPort> {
        ClosedPort::reserve()
    }

    /// Install the `aws-lc-rs` TLS provider once for the process, as a client needs before it
    /// builds a connector. Calling it again does nothing.
    pub fn ensure_tls_provider() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            // Err means a provider is already installed by another part of the test binary.
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        });
    }
}
