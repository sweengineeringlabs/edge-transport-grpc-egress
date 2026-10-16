//! Shared test fixtures of the gRPC egress crates.
//!
//! * [`Fixture`] and [`ClosedPort`]: a port that refuses connections and cannot be given away, and
//!   the TLS provider setup.
//! * [`PeerServer`]: an in-process gRPC peer that answers over HTTP/2 and records what it received.
//! * [`FreezableProxy`]: a TCP proxy a test can freeze, so a healthy peer stops answering.
//! * [`GrpcFrame`] and [`DecodedFrame`]: the five-byte gRPC message framing.
//! * [`TestPki`] and [`TlsServerMode`]: a throwaway certificate authority, and a TLS peer that
//!   may demand a client certificate.
//!
//! A test never names a port: a fixed port is only closed on the machines where nothing happens to
//! listen on it, and a port that was merely free for an instant can be handed to another listener.

mod frame;
mod peer;
mod proxy;
mod setup;
mod tls;

pub use frame::{DecodedFrame, GrpcFrame};
pub use peer::{PeerBehaviour, PeerServer, RecordedRequest, RunningPeer};
pub use proxy::{FreezableProxy, RunningProxy};
pub use setup::{ClosedPort, Fixture};
pub use tls::{TestPki, TlsServerMode};
