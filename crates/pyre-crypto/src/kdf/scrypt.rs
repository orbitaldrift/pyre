use rand_core::{CryptoRngCore, OsRng};
use scrypt::{password_hash::SaltString, scrypt};

use crate::kdf::Kdf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScryptKdfParams {
    /// log2(n) of the number of iterations
    pub n: u8,
    /// Block size
    pub r: u32,
    /// Parallelism
    pub p: u32,
    /// Derived key length
    pub dklen: usize,
    /// Salt used when deriving the key
    pub salt: Vec<u8>,
}

impl ScryptKdfParams {
    pub const BLOCK_SIZE: u32 = 8;

    pub const FAST_LOG_N: u8 = 12;
    pub const FAST_PARALLELISM: u32 = 6;

    pub const SECURE_LOG_N: u8 = 18;
    pub const SECURE_PARALLELISM: u32 = 1;
}

impl From<ScryptKdfParams> for scrypt::Params {
    fn from(params: ScryptKdfParams) -> Self {
        scrypt::Params::new(params.n, params.r, params.p, params.dklen).unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScryptKdf {
    params: ScryptKdfParams,
}

impl Default for ScryptKdf {
    fn default() -> Self {
        Self::secure(&mut OsRng)
    }
}

impl Kdf for ScryptKdf {
    const NAME: &'static str = "scrypt";

    fn fast(mut rng: impl CryptoRngCore) -> Self {
        Self::new(Self::Params {
            n: ScryptKdfParams::FAST_LOG_N,
            r: ScryptKdfParams::BLOCK_SIZE,
            p: ScryptKdfParams::FAST_PARALLELISM,
            dklen: 32,
            salt: SaltString::generate(&mut rng).as_ref().as_bytes().to_vec(),
        })
    }

    fn secure(mut rng: impl CryptoRngCore) -> Self {
        Self::new(Self::Params {
            n: ScryptKdfParams::SECURE_LOG_N,
            r: ScryptKdfParams::BLOCK_SIZE,
            p: ScryptKdfParams::SECURE_PARALLELISM,
            dklen: 32,
            salt: SaltString::generate(&mut rng).as_ref().as_bytes().to_vec(),
        })
    }

    fn secure_with_salt(salt: Vec<u8>) -> Self {
        Self::new(Self::Params {
            n: ScryptKdfParams::SECURE_LOG_N,
            r: ScryptKdfParams::BLOCK_SIZE,
            p: ScryptKdfParams::SECURE_PARALLELISM,
            dklen: 32,
            salt,
        })
    }

    type Error = scrypt::errors::InvalidOutputLen;
    type Params = ScryptKdfParams;

    fn new(params: Self::Params) -> Self {
        Self { params }
    }

    fn derive_key(&self, password: &[u8], out: &mut [u8]) -> Result<(), Self::Error> {
        let salt = self.params.salt.clone();
        scrypt(password, salt.as_slice(), &self.params.clone().into(), out)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use const_hex::ToHexExt;
    use rand::{prelude::StdRng, SeedableRng};

    use super::*;

    #[test]
    fn test_scrypt_kdf() {
        let kdf = ScryptKdf::fast(&mut StdRng::seed_from_u64(0));
        let mut key = [0u8; 32];
        kdf.derive_key(b"password", &mut key).unwrap();

        assert_eq!(
            key.encode_hex_with_prefix(),
            "0x5ca8e8322ab4b64069e816acbdbc1d9387684f9972994d0c8187f049aad1be4d"
        );
    }
}
