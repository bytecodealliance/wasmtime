use super::*;
use state::*;

use crate::rand::SecureRandom;
use ::aes_gcm::aead::{generic_array::GenericArray, AeadInPlace, NewAead};
use ::aes_gcm::{Aes128Gcm, Aes256Gcm, AesGcm};
use byteorder::{ByteOrder, LittleEndian};
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

pub const NONCE_LEN: usize = 12;
pub const TAG_LEN: usize = 16;

#[allow(clippy::large_enum_variant)]
enum AesGcmVariant {
    Aes128(Aes128Gcm),
    Aes256(Aes256Gcm),
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct AesGcmSymmetricState {
    pub alg: SymmetricAlgorithm,
    options: SymmetricOptions,
    #[derivative(Debug = "ignore")]
    ctx: AesGcmVariant,
    ad: Vec<u8>,
    nonce: Option<[u8; NONCE_LEN]>,
}

#[derive(Clone, Debug, Eq)]
pub struct AesGcmSymmetricKey {
    alg: SymmetricAlgorithm,
    raw: Vec<u8>,
}

impl PartialEq for AesGcmSymmetricKey {
    fn eq(&self, other: &Self) -> bool {
        self.alg == other.alg && self.raw.ct_eq(&other.raw).unwrap_u8() == 1
    }
}

impl Drop for AesGcmSymmetricKey {
    fn drop(&mut self) {
        self.raw.zeroize();
    }
}

impl SymmetricKeyLike for AesGcmSymmetricKey {
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

impl AesGcmSymmetricKey {
    fn new(alg: SymmetricAlgorithm, raw: &[u8]) -> Result<Self, CryptoError> {
        Ok(AesGcmSymmetricKey {
            alg,
            raw: raw.to_vec(),
        })
    }
}

pub struct AesGcmSymmetricKeyBuilder {
    alg: SymmetricAlgorithm,
}

impl AesGcmSymmetricKeyBuilder {
    pub fn new(alg: SymmetricAlgorithm) -> Box<dyn SymmetricKeyBuilder> {
        Box::new(Self { alg })
    }
}

impl SymmetricKeyBuilder for AesGcmSymmetricKeyBuilder {
    fn generate(&self, _options: Option<SymmetricOptions>) -> Result<SymmetricKey, CryptoError> {
        let mut rng = SecureRandom::new();
        let mut raw = vec![0u8; self.key_len()?];
        rng.fill(&mut raw)?;
        self.import(&raw)
    }

    fn import(&self, raw: &[u8]) -> Result<SymmetricKey, CryptoError> {
        let key = AesGcmSymmetricKey::new(self.alg, raw)?;
        Ok(SymmetricKey::new(Box::new(key)))
    }

    fn key_len(&self) -> Result<usize, CryptoError> {
        match self.alg {
            SymmetricAlgorithm::Aes128Gcm => Ok(16),
            SymmetricAlgorithm::Aes256Gcm => Ok(32),
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        }
    }
}

impl AesGcmSymmetricState {
    pub fn new(
        alg: SymmetricAlgorithm,
        key: Option<SymmetricKey>,
        options: Option<SymmetricOptions>,
    ) -> Result<Self, CryptoError> {
        let key = key.ok_or(CryptoError::KeyRequired)?;
        let key = key.inner();
        let key = key
            .as_any()
            .downcast_ref::<AesGcmSymmetricKey>()
            .ok_or(CryptoError::InvalidKey)?;
        let options = options.as_ref().ok_or(CryptoError::NonceRequired)?;
        let inner = options.inner.lock();
        let nonce_vec = inner.nonce.as_ref().ok_or(CryptoError::NonceRequired)?;
        ensure!(nonce_vec.len() == NONCE_LEN, CryptoError::InvalidNonce);
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&nonce_vec);
        let aes_gcm_impl = match alg {
            SymmetricAlgorithm::Aes128Gcm => {
                AesGcmVariant::Aes128(Aes128Gcm::new(GenericArray::from_slice(key.as_raw()?)))
            }
            SymmetricAlgorithm::Aes256Gcm => {
                AesGcmVariant::Aes256(Aes256Gcm::new(GenericArray::from_slice(key.as_raw()?)))
            }
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        };
        let state = AesGcmSymmetricState {
            alg,
            options: options.clone(),
            ctx: aes_gcm_impl,
            ad: vec![],
            nonce: Some(nonce),
        };
        Ok(state)
    }
}

impl SymmetricStateLike for AesGcmSymmetricState {
    fn alg(&self) -> SymmetricAlgorithm {
        self.alg
    }

    fn options_get(&self, name: &str) -> Result<Vec<u8>, CryptoError> {
        self.options.get(name)
    }

    fn options_get_u64(&self, name: &str) -> Result<u64, CryptoError> {
        self.options.get_u64(name)
    }

    fn absorb(&mut self, data: &[u8]) -> Result<(), CryptoError> {
        self.ad.extend_from_slice(data);
        Ok(())
    }

    fn max_tag_len(&mut self) -> Result<usize, CryptoError> {
        Ok(TAG_LEN)
    }

    fn encrypt_unchecked(&mut self, out: &mut [u8], data: &[u8]) -> Result<usize, CryptoError> {
        let data_len = data.len();
        let tag = self.encrypt_detached_unchecked(&mut out[..data_len], data)?;
        out[data_len..].copy_from_slice(tag.as_ref());
        Ok(out.len())
    }

    fn encrypt_detached_unchecked(
        &mut self,
        out: &mut [u8],
        data: &[u8],
    ) -> Result<SymmetricTag, CryptoError> {
        let nonce = self.nonce.as_ref().ok_or(CryptoError::NonceRequired)?;
        if out.as_ptr() != data.as_ptr() {
            out.copy_from_slice(data);
        }
        let raw_tag = match &self.ctx {
            AesGcmVariant::Aes128(x) => {
                x.encrypt_in_place_detached(GenericArray::from_slice(nonce), &self.ad, out)
            }
            AesGcmVariant::Aes256(x) => {
                x.encrypt_in_place_detached(GenericArray::from_slice(nonce), &self.ad, out)
            }
        }
        .map_err(|_| CryptoError::InternalError)?
        .to_vec();

        self.nonce = None;
        Ok(SymmetricTag::new(self.alg, raw_tag))
    }

    fn decrypt_unchecked(&mut self, out: &mut [u8], data: &[u8]) -> Result<usize, CryptoError> {
        let raw_tag = &data[out.len()..].to_vec();
        self.decrypt_detached_unchecked(out, &data[..out.len()], &raw_tag)
    }

    fn decrypt_detached_unchecked(
        &mut self,
        out: &mut [u8],
        data: &[u8],
        raw_tag: &[u8],
    ) -> Result<usize, CryptoError> {
        let nonce = self.nonce.as_ref().ok_or(CryptoError::NonceRequired)?;
        if out.as_ptr() != data.as_ptr() {
            out[..data.len()].copy_from_slice(data);
        }
        match &self.ctx {
            AesGcmVariant::Aes128(x) => x.decrypt_in_place_detached(
                GenericArray::from_slice(nonce),
                &self.ad,
                out,
                GenericArray::from_slice(raw_tag),
            ),
            AesGcmVariant::Aes256(x) => x.decrypt_in_place_detached(
                GenericArray::from_slice(nonce),
                &self.ad,
                out,
                GenericArray::from_slice(raw_tag),
            ),
        }
        .map_err(|_| CryptoError::InvalidTag)?;
        Ok(data.len())
    }
}
