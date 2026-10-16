//! TLS for a peer under test: a throwaway certificate authority and what a peer demands of clients.

mod test_pki;
mod tls_server_mode;

pub use test_pki::TestPki;
pub use tls_server_mode::TlsServerMode;
