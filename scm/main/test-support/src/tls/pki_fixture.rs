//! [`TestPki`] -- a throwaway certificate authority with a server and a client certificate.

use std::io;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};

use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose, SanType,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tempfile::TempDir;

/// A certificate authority generated for one test, the certificate it signed for the peer's
/// server (valid for `127.0.0.1` and `localhost`), and the one it signed for a client.
///
/// Every certificate and key is also written as PEM into a directory that lives as long as the
/// value, because the code under test reads them from paths.
pub struct TestPki {
    dir: TempDir,
    server_chain: Vec<CertificateDer<'static>>,
    server_key: PrivateKeyDer<'static>,
    client_root: Vec<CertificateDer<'static>>,
}

impl TestPki {
    /// Generate an authority and its two certificates.
    ///
    /// # Errors
    ///
    /// The error of generating a key or certificate, or of writing the PEM files.
    pub fn create() -> io::Result<Self> {
        let ca_key = Self::generate_key()?;
        let mut ca_params = Self::new_params(Vec::new())?;
        ca_params
            .distinguished_name
            .push(DnType::CommonName, "grpc-egress test authority");
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let ca_cert = ca_params.self_signed(&ca_key).map_err(io::Error::other)?;
        let issuer = Issuer::new(ca_params, ca_key);

        let server_key = Self::generate_key()?;
        let mut server_params = Self::new_params(vec!["localhost".to_string()])?;
        server_params
            .subject_alt_names
            .push(SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let server_cert = server_params
            .signed_by(&server_key, &issuer)
            .map_err(io::Error::other)?;

        let client_key = Self::generate_key()?;
        let mut client_params = Self::new_params(Vec::new())?;
        client_params
            .distinguished_name
            .push(DnType::CommonName, "grpc-egress test client");
        client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
        let client_cert = client_params
            .signed_by(&client_key, &issuer)
            .map_err(io::Error::other)?;

        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("ca.pem"), ca_cert.pem())?;
        std::fs::write(dir.path().join("server.pem"), server_cert.pem())?;
        std::fs::write(dir.path().join("server.key"), server_key.serialize_pem())?;
        std::fs::write(dir.path().join("client.pem"), client_cert.pem())?;
        std::fs::write(dir.path().join("client.key"), client_key.serialize_pem())?;

        Ok(Self {
            dir,
            server_chain: vec![server_cert.der().clone()],
            server_key: PrivateKeyDer::try_from(server_key.serialize_der())
                .map_err(io::Error::other)?,
            client_root: vec![ca_cert.der().clone()],
        })
    }

    /// The PEM file of the authority's certificate (what a client trusts).
    pub fn get_ca_pem_path(&self) -> PathBuf {
        self.dir.path().join("ca.pem")
    }

    /// The PEM file of the certificate the authority signed for a client.
    pub fn get_client_cert_pem_path(&self) -> PathBuf {
        self.dir.path().join("client.pem")
    }

    /// The PEM file of the private key of the client certificate.
    pub fn get_client_key_pem_path(&self) -> PathBuf {
        self.dir.path().join("client.key")
    }

    /// The directory holding every PEM file.
    pub fn get_dir(&self) -> &Path {
        self.dir.path()
    }

    /// The server certificate chain, for a peer's TLS configuration.
    pub fn get_server_chain(&self) -> Vec<CertificateDer<'static>> {
        self.server_chain.clone()
    }

    /// The private key of the server certificate, for a peer's TLS configuration.
    pub fn get_server_key(&self) -> PrivateKeyDer<'static> {
        self.server_key.clone_key()
    }

    /// The authority's certificate, which a peer trusts to sign client certificates.
    pub fn get_client_roots(&self) -> Vec<CertificateDer<'static>> {
        self.client_root.clone()
    }

    fn generate_key() -> io::Result<KeyPair> {
        KeyPair::generate().map_err(io::Error::other)
    }

    fn new_params(names: Vec<String>) -> io::Result<CertificateParams> {
        CertificateParams::new(names).map_err(io::Error::other)
    }
}
