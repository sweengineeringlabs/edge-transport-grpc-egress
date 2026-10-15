//! What a test sets up before it starts: a held closed port and the TLS provider.

mod closed_port;
mod fixture;

pub use closed_port::ClosedPort;
pub use fixture::Fixture;
