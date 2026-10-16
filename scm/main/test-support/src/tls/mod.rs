//! TLS for a peer under test: a throwaway certificate authority and what a peer demands of clients.

mod pki_fixture;
mod tls_server_mode;

pub use pki_fixture::TestPki;
pub use tls_server_mode::TlsServerMode;
