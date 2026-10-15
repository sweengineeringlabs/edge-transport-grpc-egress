//! Shared test fixtures of the gRPC egress crates.
//!
//! * [`Fixture`] and [`ClosedPort`]: a port that refuses connections and cannot be given away, and
//!   the TLS provider setup.
//! * [`PeerServer`]: an in-process gRPC peer that answers over HTTP/2 and records what it received.
//! * [`GrpcFrame`] and [`DecodedFrame`]: the five-byte gRPC message framing.
//!
//! A test never names a port: a fixed port is only closed on the machines where nothing happens to
//! listen on it, and a port that was merely free for an instant can be handed to another listener.

mod closed_port;
mod decoded_frame;
mod fixture;
mod grpc_frame;
mod peer_behaviour;
mod peer_server;
mod recorded_request;
mod running_peer;

pub use closed_port::ClosedPort;
pub use decoded_frame::DecodedFrame;
pub use fixture::Fixture;
pub use grpc_frame::GrpcFrame;
pub use peer_behaviour::PeerBehaviour;
pub use peer_server::PeerServer;
pub use recorded_request::RecordedRequest;
pub use running_peer::RunningPeer;
