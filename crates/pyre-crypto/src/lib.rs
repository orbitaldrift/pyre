#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid pem key: {0}")]
    InvalidKey(String),

    #[error("rustls error: {0}")]
    RustlsError(#[from] rustls::Error),
}

pub struct TlsServerConfig(pub rustls::ServerConfig);

impl TlsServerConfig {
    /// Creates a new `TlsServerConfig` with the given `PkiCert` and ALPN protocols.
    /// The `PkiCert` should contain a valid certificate and private key.
    ///
    /// # Errors
    /// Fails on certificate errors.
    pub fn new(pki: &PkiCert, protocols: Vec<Vec<u8>>) -> Result<Self, Error> {
        let mut tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![pki.cert.clone()], pki.key.clone_key())?;

        tls_config.alpn_protocols = protocols;
        tls_config.max_early_data_size = u32::MAX;

        Ok(TlsServerConfig(tls_config))
    }
}

impl From<TlsServerConfig> for rustls::ServerConfig {
    fn from(value: TlsServerConfig) -> Self {
        value.0
    }
}

pub struct PkiCert {
    pub cert: rustls::pki_types::CertificateDer<'static>,
    pub key: rustls::pki_types::PrivateKeyDer<'static>,
}

impl PkiCert {
    pub fn from_bytes(cert: Vec<u8>, key: Vec<u8>) -> Result<Self, Error> {
        let cert = rustls::pki_types::CertificateDer::from(cert);
        let key = rustls::pki_types::PrivateKeyDer::try_from(key)
            .map_err(|e| Error::InvalidKey(e.to_string()))?;

        Ok(PkiCert { cert, key })
    }
}
