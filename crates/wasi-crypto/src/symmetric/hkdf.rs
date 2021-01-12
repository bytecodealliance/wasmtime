use super::state::*;
use super::*;
use crate::rand::SecureRandom;

use ::hkdf::Hkdf;
use ::sha2::{Sha256, Sha512};
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

#[derive(Clone, Debug)]
pub struct HkdfSymmetricState {
    pub alg: SymmetricAlgorithm,
    options: Option<SymmetricOptions>,
    key: Vec<u8>,
    data: Vec<u8>,
}

impl Drop for HkdfSymmetricState {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl SymmetricKeyLike for HkdfSymmetricKey {
    fn alg(&self) -> SymmetricAlgorithm {
        self.alg
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_raw(&self) -> Result<&[u8], CryptoError> {
        Ok(&self.raw)
    }
}

#[derive(Clone, Debug, Eq)]
pub struct HkdfSymmetricKey {
    alg: SymmetricAlgorithm,
    raw: Vec<u8>,
}

impl Drop for HkdfSymmetricKey {
    fn drop(&mut self) {
        self.raw.zeroize();
    }
}

impl PartialEq for HkdfSymmetricKey {
    fn eq(&self, other: &Self) -> bool {
        self.alg == other.alg && self.raw.ct_eq(&other.raw).unwrap_u8() == 1
    }
}

impl HkdfSymmetricKey {
    pub fn new(alg: SymmetricAlgorithm, raw: &[u8]) -> Result<Self, CryptoError> {
        Ok(HkdfSymmetricKey {
            alg,
            raw: raw.to_vec(),
        })
    }
}

pub struct HkdfSymmetricKeyBuilder {
    alg: SymmetricAlgorithm,
}

impl HkdfSymmetricKeyBuilder {
    pub fn new(alg: SymmetricAlgorithm) -> Box<dyn SymmetricKeyBuilder> {
        Box::new(Self { alg })
    }
}

impl SymmetricKeyBuilder for HkdfSymmetricKeyBuilder {
    fn generate(&self, _options: Option<SymmetricOptions>) -> Result<SymmetricKey, CryptoError> {
        let mut rng = SecureRandom::new();
        let mut raw = vec![0u8; self.key_len()?];
        rng.fill(&mut raw)?;
        self.import(&raw)
    }

    fn import(&self, raw: &[u8]) -> Result<SymmetricKey, CryptoError> {
        let key = HkdfSymmetricKey::new(self.alg, raw)?;
        Ok(SymmetricKey::new(Box::new(key)))
    }

    fn key_len(&self) -> Result<usize, CryptoError> {
        match self.alg {
            SymmetricAlgorithm::HkdfSha256Expand | SymmetricAlgorithm::HkdfSha256Extract => Ok(32),
            SymmetricAlgorithm::HkdfSha512Expand | SymmetricAlgorithm::HkdfSha512Extract => Ok(64),
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        }
    }
}

impl HkdfSymmetricState {
    pub fn new(
        alg: SymmetricAlgorithm,
        key: Option<SymmetricKey>,
        options: Option<SymmetricOptions>,
    ) -> Result<Self, CryptoError> {
        let key = key.ok_or(CryptoError::KeyRequired)?;
        let key = key.inner();
        let key = key
            .as_any()
            .downcast_ref::<HkdfSymmetricKey>()
            .ok_or(CryptoError::InvalidKey)?;
        let key = key.as_raw()?.to_vec();
        Ok(HkdfSymmetricState {
            alg,
            options,
            key,
            data: vec![],
        })
    }
}

impl SymmetricStateLike for HkdfSymmetricState {
    fn alg(&self) -> SymmetricAlgorithm {
        self.alg
    }

    fn options_get(&self, name: &str) -> Result<Vec<u8>, CryptoError> {
        self.options
            .as_ref()
            .ok_or(CryptoError::OptionNotSet)?
            .get(name)
    }

    fn options_get_u64(&self, name: &str) -> Result<u64, CryptoError> {
        self.options
            .as_ref()
            .ok_or(CryptoError::OptionNotSet)?
            .get_u64(name)
    }

    fn absorb(&mut self, data: &[u8]) -> Result<(), CryptoError> {
        self.data.extend_from_slice(data);
        Ok(())
    }

    fn squeeze_key(&mut self, alg_str: &str) -> Result<SymmetricKey, CryptoError> {
        let raw_prk = match self.alg {
            SymmetricAlgorithm::HkdfSha256Extract => {
                Hkdf::<Sha256>::extract(Some(&self.data), &self.key)
                    .0
                    .to_vec()
            }
            SymmetricAlgorithm::HkdfSha512Extract => {
                Hkdf::<Sha512>::extract(Some(&self.data), &self.key)
                    .0
                    .to_vec()
            }
            _ => bail!(CryptoError::InvalidOperation),
        };
        let builder = SymmetricKey::builder(alg_str)?;
        builder.import(&raw_prk)
    }

    fn squeeze(&mut self, out: &mut [u8]) -> Result<(), CryptoError> {
        match self.alg {
            SymmetricAlgorithm::HkdfSha256Expand => Hkdf::<Sha256>::from_prk(&self.key)
                .map_err(|_| CryptoError::InvalidKey)?
                .expand(&self.data, out),
            SymmetricAlgorithm::HkdfSha512Expand => Hkdf::<Sha512>::from_prk(&self.key)
                .map_err(|_| CryptoError::InvalidKey)?
                .expand(&self.data, out),
            _ => bail!(CryptoError::InvalidOperation),
        }
        .map_err(|_| CryptoError::Overflow)
    }
}
