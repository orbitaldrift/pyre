use rcgen::{generate_simple_self_signed, CertifiedKey};

use crate::kdf::{scrypt::ScryptKdf, Kdf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid pem key: {0}")]
    InvalidKey(String),

    #[error("rustls error: {0}")]
    Rustls(#[from] rustls::Error),

    #[error("rcgen error: {0}")]
    Rcgen(#[from] rcgen::Error),
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

    /// Creates a new `PkiCert` from a rcgen generated self-signed certificate.
    ///
    /// # Errors
    /// Fails if the key is invalid, not Pkcs or Sec1.
    pub fn new_self_signed() -> Result<Self, Error> {
        let CertifiedKey { cert, key_pair } =
            generate_simple_self_signed(vec!["localhost".to_string()])?;

        Self::from_bytes(cert.der().to_vec(), key_pair.serialize_der())
    }

    #[must_use]
    /// Derives a key from the given password and salt using the Scrypt KDF.
    ///
    /// # Panics
    /// Panics if the key derivation fails on invalid output length.
    pub fn derive_key(&self, salt: Vec<u8>) -> [u8; 64] {
        let kdf = ScryptKdf::secure_with_salt(salt);
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
        let kdf = ScryptKdf::fast(rand::thread_rng());
        let mut out = [0u8; 64];

        kdf.derive_key(self.key.secret_der(), &mut out)
            .expect("failed to derive key");

        out
    }
}

#[cfg(test)]
mod tests {
    use rand::RngCore;

    use super::*;

    #[test]
    fn test_derive_fast() {
        let mut salt = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut salt);

        let pki = PkiCert::new_self_signed().unwrap();
        let key = pki.derive_key_fast();

        assert_eq!(key.len(), 64);
    }
}
