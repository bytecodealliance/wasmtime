use super::state::*;
use super::*;
use crate::rand::SecureRandom;

use ::sha2::{Sha256, Sha512};
use hmac::{Hmac, Mac, NewMac};
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum HmacVariant {
    Sha256(Hmac<Sha256>),
    Sha512(Hmac<Sha512>),
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct HmacSha2SymmetricState {
    pub alg: SymmetricAlgorithm,
    options: Option<SymmetricOptions>,
    ctx: HmacVariant,
}

#[derive(Clone, Debug, Eq)]
pub struct HmacSha2SymmetricKey {
    alg: SymmetricAlgorithm,
    raw: Vec<u8>,
}

impl PartialEq for HmacSha2SymmetricKey {
    fn eq(&self, other: &Self) -> bool {
        self.alg == other.alg && self.raw.ct_eq(&other.raw).unwrap_u8() == 1
    }
}

impl Drop for HmacSha2SymmetricKey {
    fn drop(&mut self) {
        self.raw.zeroize();
    }
}

impl SymmetricKeyLike for HmacSha2SymmetricKey {
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

impl HmacSha2SymmetricKey {
    pub fn new(alg: SymmetricAlgorithm, raw: &[u8]) -> Result<Self, CryptoError> {
        Ok(HmacSha2SymmetricKey {
            alg,
            raw: raw.to_vec(),
        })
    }
}

pub struct HmacSha2SymmetricKeyBuilder {
    alg: SymmetricAlgorithm,
}

impl HmacSha2SymmetricKeyBuilder {
    pub fn new(alg: SymmetricAlgorithm) -> Box<dyn SymmetricKeyBuilder> {
        Box::new(Self { alg })
    }
}

impl SymmetricKeyBuilder for HmacSha2SymmetricKeyBuilder {
    fn generate(&self, _options: Option<SymmetricOptions>) -> Result<SymmetricKey, CryptoError> {
        let mut rng = SecureRandom::new();
        let mut raw = vec![0u8; self.key_len()?];
        rng.fill(&mut raw)?;
        self.import(&raw)
    }

    fn import(&self, raw: &[u8]) -> Result<SymmetricKey, CryptoError> {
        let key = HmacSha2SymmetricKey::new(self.alg, raw)?;
        Ok(SymmetricKey::new(Box::new(key)))
    }

    fn key_len(&self) -> Result<usize, CryptoError> {
        match self.alg {
            SymmetricAlgorithm::HmacSha256 => Ok(32),
            SymmetricAlgorithm::HmacSha512 => Ok(64),
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        }
    }
}

impl HmacSha2SymmetricState {
    pub fn new(
        alg: SymmetricAlgorithm,
        key: Option<SymmetricKey>,
        options: Option<SymmetricOptions>,
    ) -> Result<Self, CryptoError> {
        let key = key.ok_or(CryptoError::KeyRequired)?;
        let key = key.inner();
        let key = key
            .as_any()
            .downcast_ref::<HmacSha2SymmetricKey>()
            .ok_or(CryptoError::InvalidKey)?;
        let ctx = match alg {
            SymmetricAlgorithm::HmacSha256 => HmacVariant::Sha256(
                Hmac::<Sha256>::new_varkey(key.as_raw()?).map_err(|_| CryptoError::InvalidKey)?,
            ),
            SymmetricAlgorithm::HmacSha512 => HmacVariant::Sha512(
                Hmac::<Sha512>::new_varkey(key.as_raw()?).map_err(|_| CryptoError::InvalidKey)?,
            ),
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        };
        Ok(HmacSha2SymmetricState { alg, options, ctx })
    }
}

impl SymmetricStateLike for HmacSha2SymmetricState {
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
        match &mut self.ctx {
            HmacVariant::Sha256(x) => x.update(data),
            HmacVariant::Sha512(x) => x.update(data),
        };
        Ok(())
    }

    fn squeeze_tag(&mut self) -> Result<SymmetricTag, CryptoError> {
        let raw = match &self.ctx {
            HmacVariant::Sha256(x) => x.clone().finalize().into_bytes().to_vec(),
            HmacVariant::Sha512(x) => x.clone().finalize().into_bytes().to_vec(),
        };
        Ok(SymmetricTag::new(self.alg, raw))
    }
}
