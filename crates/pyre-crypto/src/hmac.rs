use base64::Engine;
use hmac::{Hmac, Mac};
use secstr::SecStr;
use sha2::Sha256;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    InvalidKeyLength(#[from] hmac::digest::InvalidLength),
}

#[derive(Clone)]
pub struct Base64Hmac<'a> {
    pub key: &'a SecStr,
}

impl<'a> Base64Hmac<'a> {
    #[must_use]
    pub fn new(key: &'a SecStr) -> Self {
        Base64Hmac { key }
    }

    /// Sign a message using HMAC with SHA256 and return the result as a base64 encoded string.
    ///
    /// # Errors
    /// If the key length is invalid, an `Error::InvalidKeyLength` error is returned.
    pub fn sign(&self, base64_engine: &impl Engine, message: &[u8]) -> Result<String, Error> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.key.unsecure())?;
        mac.update(message);
        Ok(base64_engine.encode(mac.finalize().into_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use base64::prelude::BASE64_URL_SAFE_NO_PAD;
    use secstr::SecStr;

    use super::*;

    #[test]
    fn test_hmac_sign() {
        let key = SecStr::from("my_secret_key");
        let message = b"my_message";
        let hmac = Base64Hmac::new(&key);
        let signature = hmac.sign(&BASE64_URL_SAFE_NO_PAD, message).unwrap();
        let expected_signature = "3RKWN4LEX0xGcIvKbVPJYx0r-9U7DghtdlErOMuHFb4";
        assert_eq!(signature, expected_signature);
    }
}
