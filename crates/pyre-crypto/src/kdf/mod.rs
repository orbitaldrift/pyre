use rand_core::CryptoRngCore;

pub mod scrypt;

pub trait Kdf: Sized {
    const NAME: &'static str;

    fn fast(rng: impl CryptoRngCore) -> Self;
    fn secure(rng: impl CryptoRngCore) -> Self;
    fn secure_with_salt(salt: Vec<u8>) -> Self;

    type Error: std::error::Error;
    type Params;

    fn new(params: Self::Params) -> Self;

    /// Derives a key from the given password and salt using the Scrypt KDF.
    ///
    /// # Errors
    /// If the key derivation fails on invalid output length.
    fn derive_key(&self, password: &[u8], out: &mut [u8]) -> Result<(), Self::Error>;
}
