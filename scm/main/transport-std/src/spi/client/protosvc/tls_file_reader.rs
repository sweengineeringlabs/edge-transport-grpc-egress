//! `TlsFileReader` -- the PEM files named by the TLS options, read into the contract's `TlsSettings`.

use proto_pattern::{TlsIdentity, TlsSettings};

use grpc_egress_transport::{GrpcChannelConfigError, MtlsConfig};

const CERTIFICATE_MARKER: &str = "-----BEGIN CERTIFICATE-----";
const PRIVATE_KEY_MARKER: &str = "PRIVATE KEY-----";

/// Reads the certificate, key and CA files the options name.
///
/// The library of the engine reads no path, so the files are read here, once, when the client is
/// built: a file that cannot be read or holds no PEM item of the kind expected is an error naming its
/// path, found at build and not at the first call. The engine checks that the items are well formed.
pub(crate) struct TlsFileReader;

impl TlsFileReader {
    /// The TLS settings for `identity` and `root_ca`; with neither, the system's roots are trusted.
    ///
    /// # Errors
    ///
    /// `UnusableTlsFile` for the first file that cannot be used.
    pub(crate) fn read(
        identity: Option<&MtlsConfig>,
        root_ca: Option<&str>,
    ) -> Result<TlsSettings, GrpcChannelConfigError> {
        let roots = match root_ca {
            Some(path) => vec![Self::read_pem(
                path,
                CERTIFICATE_MARKER,
                "no certificate found in the file",
            )?],
            None => Vec::new(),
        };
        let identity = match identity {
            Some(mtls) => Some(TlsIdentity::create(
                Self::read_pem(
                    &mtls.cert_pem_path,
                    CERTIFICATE_MARKER,
                    "no certificate found in the file",
                )?,
                Self::read_pem(
                    &mtls.key_pem_path,
                    PRIVATE_KEY_MARKER,
                    "no private key found in the file",
                )?,
            )),
            None => None,
        };
        Ok(TlsSettings::create(None, roots, identity))
    }

    /// The bytes of the file at `path`, which must hold `marker`.
    fn read_pem(
        path: &str,
        marker: &str,
        missing: &str,
    ) -> Result<Vec<u8>, GrpcChannelConfigError> {
        let bytes = std::fs::read(path).map_err(|e| Self::refuse(path, e))?;
        let text = String::from_utf8_lossy(&bytes);
        if !text.contains(marker) {
            return Err(Self::refuse(path, missing));
        }
        Ok(bytes)
    }

    fn refuse(path: &str, reason: impl std::fmt::Display) -> GrpcChannelConfigError {
        GrpcChannelConfigError::UnusableTlsFile {
            path: path.to_string(),
            reason: reason.to_string(),
        }
    }
}
