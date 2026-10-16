//! `TonicTlsConnector` — the TLS side of the client: trusted roots and the client identity.

use hyper_util::client::legacy::connect::HttpConnector;
use rustls::pki_types::pem::PemObject as _;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

use crate::api::{GrpcChannelConfigError, MtlsConfig};

/// Builds the HTTPS connector of the client from the TLS options.
///
/// Without a pinned CA the server is verified against the bundled web PKI roots; with one it is
/// verified against that CA alone. With an identity the client presents its certificate chain
/// (mutual TLS). A file that cannot be used is an error naming its path, found when the client is
/// built and not at the first call.
pub(crate) struct TonicTlsConnector;

impl TonicTlsConnector {
    /// Build the connector for `identity` and `pinned_ca`.
    pub(crate) fn create(
        identity: Option<&MtlsConfig>,
        pinned_ca: Option<&str>,
    ) -> Result<hyper_rustls::HttpsConnector<HttpConnector>, GrpcChannelConfigError> {
        // rustls 0.23 needs a process-wide provider before the first ClientConfig; idempotent.
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let roots = match pinned_ca {
            Some(path) => Self::read_roots(path)?,
            None => rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
            },
        };
        let builder = rustls::ClientConfig::builder().with_root_certificates(roots);
        let config = match identity {
            Some(mtls) => {
                let chain = Self::read_chain(&mtls.cert_pem_path)?;
                let key = Self::read_key(&mtls.key_pem_path)?;
                builder
                    .with_client_auth_cert(chain, key)
                    .map_err(|e| Self::refuse(&mtls.key_pem_path, e))?
            }
            None => builder.with_no_client_auth(),
        };
        Ok(hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(config)
            .https_or_http()
            .enable_http2()
            .build())
    }

    fn read_roots(path: &str) -> Result<rustls::RootCertStore, GrpcChannelConfigError> {
        let mut roots = rustls::RootCertStore::empty();
        for cert in Self::read_chain(path)? {
            roots.add(cert).map_err(|e| Self::refuse(path, e))?;
        }
        Ok(roots)
    }

    fn read_chain(path: &str) -> Result<Vec<CertificateDer<'static>>, GrpcChannelConfigError> {
        let iter = CertificateDer::pem_file_iter(path).map_err(|e| Self::refuse(path, e))?;
        let chain = iter
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| Self::refuse(path, e))?;
        if chain.is_empty() {
            return Err(Self::refuse(path, "no certificate found in the file"));
        }
        Ok(chain)
    }

    fn read_key(path: &str) -> Result<PrivateKeyDer<'static>, GrpcChannelConfigError> {
        PrivateKeyDer::from_pem_file(path).map_err(|e| Self::refuse(path, e))
    }

    fn refuse(path: &str, reason: impl std::fmt::Display) -> GrpcChannelConfigError {
        GrpcChannelConfigError::UnusableTlsFile {
            path: path.to_string(),
            reason: reason.to_string(),
        }
    }
}
