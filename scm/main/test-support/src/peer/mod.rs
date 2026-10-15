//! An in-process gRPC peer: how it answers, what it received, and the running server.

mod peer_behaviour;
mod peer_server;
mod recorded_request;
mod running_peer;

pub use peer_behaviour::PeerBehaviour;
pub use peer_server::PeerServer;
pub use recorded_request::RecordedRequest;
pub use running_peer::RunningPeer;
