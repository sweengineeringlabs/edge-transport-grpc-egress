//! [`TlsServerMode`] -- what a TLS peer demands of the client that connects.

/// What a TLS [`PeerServer`](crate::PeerServer) asks of a client during the handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsServerMode {
    /// The peer proves itself; any client may connect.
    ServerAuthOnly,
    /// The peer also demands a client certificate signed by its own authority (mutual TLS).
    ClientAuthRequired,
}
