use rcgen::{generate_simple_self_signed, CertifiedKey};
use rustls::pki_types::pem::PemObject;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("pem serialization error (testcert): {0}")]
    PemSerialization(String),

    #[error("rustls error: {0}")]
    RustlsError(#[from] rustls::Error),
}

pub struct TlsServerConfig(pub rustls::ServerConfig);

impl TlsServerConfig {
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
    pub fn new(subject_alt_names: Vec<String>) -> Result<Self, Error> {
        let (cert, keypair) = Self::test_cert(subject_alt_names.clone());
        let cert = rustls::pki_types::CertificateDer::from(cert);
        let key = rustls::pki_types::PrivateKeyDer::from_pem(
            rustls::pki_types::pem::SectionKind::PrivateKey,
            keypair.serialize_der(),
        )
        .ok_or(Error::PemSerialization(
            subject_alt_names.join(",").to_string(),
        ))?;

        Ok(PkiCert { cert, key })
    }

    fn test_cert(subject_alt_names: Vec<String>) -> (rcgen::Certificate, rcgen::KeyPair) {
        let CertifiedKey { cert, key_pair } =
            generate_simple_self_signed(subject_alt_names).unwrap();
        (cert, key_pair)
    }
}
