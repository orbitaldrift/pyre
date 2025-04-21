use std::ops::Deref;

use base64::{prelude::BASE64_STANDARD, Engine};
use pyre_crypto::hmac::Base64Hmac;
use secstr::SecStr;

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct CsrfToken(pub String);

impl Deref for CsrfToken {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CsrfToken {
    /// Creates a new CSRF token using the provided secret and identifier.
    /// Identifier should be changing per session only, not per request.
    /// Unless a session Id has not been generated yet.
    ///
    /// # Errors
    /// If the HMAC key is invalid.
    pub fn new(secret: &SecStr, identifier: impl Into<String>) -> Result<Self, Error> {
        let message = format!(
            "{}!{}",
            identifier.into(),
            BASE64_STANDARD.encode(get_random_value())
        );
        let signature = Base64Hmac::new(secret).sign(message.as_bytes())?;
        let token = format!("{signature}.{message}");

        Ok(CsrfToken(token))
    }

    /// Validates the CSRF token using the provided secret and another token.
    ///
    /// # Errors
    /// If the HMAC key is invalid.
    pub fn validate(&self, secret: &SecStr, token: impl Into<String>) -> Result<bool, Error> {
        let token = token.into();
        let cookie = &self.0;

        let mut parts = token.splitn(2, '.');
        let received_hmac = parts.next().unwrap_or("");

        let message = parts.next().unwrap_or("");
        let expected_hmac = Base64Hmac::new(secret).sign(message.as_bytes())?;

        if !bool::from(subtle::ConstantTimeEq::ct_eq(
            cookie.as_bytes(),
            token.as_bytes(),
        )) {
            return Ok(false);
        }

        Ok(
            subtle::ConstantTimeEq::ct_eq(received_hmac.as_bytes(), expected_hmac.as_bytes())
                .into(),
        )
    }

    /// Validates a token against its own signature.
    ///
    /// # Errors
    /// If the HMAC key is invalid.
    pub fn validate_signature_only(secret: &SecStr, token: &str) -> Result<bool, Error> {
        let mut parts = token.splitn(2, '.');
        let signature = parts.next().unwrap_or("");
        let message = parts.next().unwrap_or("");

        let expected_signature = Base64Hmac::new(secret).sign(message.as_bytes())?;

        Ok(bool::from(subtle::ConstantTimeEq::ct_eq(
            signature.as_bytes(),
            expected_signature.as_bytes(),
        )))
    }
}

#[cfg(not(test))]
fn get_random_value() -> [u8; 64] {
    use rand::Rng;

    let mut random = [0u8; 64];
    rand::rng().fill(&mut random);

    random
}

#[cfg(test)]
fn get_random_value() -> [u8; 64] {
    [42u8; 64]
}

#[cfg(test)]
mod tests {
    use color_eyre::Result;

    use super::*;

    #[test]
    fn test_token() -> Result<()> {
        let identifier = "identifier";
        let secret: SecStr = "super-secret".as_bytes().to_vec().into();
        let token = CsrfToken::new(&secret, identifier)?;

        let parts = token.splitn(2, '.').collect::<Vec<&str>>();
        assert_eq!(parts.len(), 2);

        let message = format!(
            "{}!{}",
            identifier,
            BASE64_STANDARD.encode(get_random_value())
        );
        assert_eq!(parts[1], message);

        let signature = Base64Hmac::new(&secret).sign(message.as_bytes())?;
        assert_eq!(parts[0], signature);

        let valid = token.validate(&secret, (*token).clone())?;
        assert!(valid);

        let valid = CsrfToken::validate_signature_only(&secret, &token).unwrap();
        assert!(valid);

        Ok(())
    }
}
