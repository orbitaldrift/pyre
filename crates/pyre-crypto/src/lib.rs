use kdf::Kdf;

pub mod kdf;

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
    /// Creates a new `PkiCert` from the given certificate and private key.
    /// The certificate and key should be in DER format. Preferably Pkcs8.
    ///
    /// # Errors
    /// Fails if the key is invalid, not Pkcs or Sec1.
    pub fn from_bytes(cert: Vec<u8>, key: Vec<u8>) -> Result<Self, Error> {
        let cert = rustls::pki_types::CertificateDer::from(cert);
        let key = rustls::pki_types::PrivateKeyDer::try_from(key)
            .map_err(|e| Error::InvalidKey(e.to_string()))?;

        Ok(PkiCert { cert, key })
    }

    #[must_use]
    /// Derives a key from the given password and salt using the Scrypt KDF.
    ///
    /// # Panics
    /// Panics if the key derivation fails on invalid output length.
    pub fn derive_key(&self, salt: Vec<u8>) -> [u8; 64] {
        let kdf = kdf::scrypt::ScryptKdf::secure_with_salt(salt);
        let mut out = [0u8; 64];

        kdf.derive_key(self.key.secret_der(), &mut out)
            .expect("failed to derive key");

        out
    }

    #[must_use]
    /// WARNING! Unsafe to use in production.
    /// Derives a key from the given password and salt using the Scrypt KDF.
    ///
    /// # Panics
    /// Panics if the key derivation fails on invalid output length.
    pub fn derive_key_fast(&self) -> [u8; 64] {
        let kdf = kdf::scrypt::ScryptKdf::fast(rand::thread_rng());
        let mut out = [0u8; 64];

        kdf.derive_key(self.key.secret_der(), &mut out)
            .expect("failed to derive key");

        out
    }
}
