use super::*;
use state::*;

use crate::rand::SecureRandom;
use ::chacha20poly1305::aead::{generic_array::GenericArray, AeadInPlace, NewAead};
use ::chacha20poly1305::{ChaCha20Poly1305, XChaCha20Poly1305};
use byteorder::{ByteOrder, LittleEndian};
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

pub const TAG_LEN: usize = 16;

#[allow(clippy::large_enum_variant)]
enum ChaChaPolyVariant {
    ChaCha(ChaCha20Poly1305),
    XChaCha(XChaCha20Poly1305),
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct ChaChaPolySymmetricState {
    pub alg: SymmetricAlgorithm,
    options: SymmetricOptions,
    #[derivative(Debug = "ignore")]
    ctx: ChaChaPolyVariant,
    ad: Vec<u8>,
    nonce: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Eq)]
pub struct ChaChaPolySymmetricKey {
    alg: SymmetricAlgorithm,
    raw: Vec<u8>,
}

impl PartialEq for ChaChaPolySymmetricKey {
    fn eq(&self, other: &Self) -> bool {
        self.alg == other.alg && self.raw.ct_eq(&other.raw).unwrap_u8() == 1
    }
}

impl Drop for ChaChaPolySymmetricKey {
    fn drop(&mut self) {
        self.raw.zeroize();
    }
}

impl SymmetricKeyLike for ChaChaPolySymmetricKey {
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

impl ChaChaPolySymmetricKey {
    fn new(alg: SymmetricAlgorithm, raw: &[u8]) -> Result<Self, CryptoError> {
        Ok(ChaChaPolySymmetricKey {
            alg,
            raw: raw.to_vec(),
        })
    }
}

pub struct ChaChaPolySymmetricKeyBuilder {
    alg: SymmetricAlgorithm,
}

impl ChaChaPolySymmetricKeyBuilder {
    pub fn new(alg: SymmetricAlgorithm) -> Box<dyn SymmetricKeyBuilder> {
        Box::new(Self { alg })
    }
}

impl SymmetricKeyBuilder for ChaChaPolySymmetricKeyBuilder {
    fn generate(&self, _options: Option<SymmetricOptions>) -> Result<SymmetricKey, CryptoError> {
        let mut rng = SecureRandom::new();
        let mut raw = vec![0u8; self.key_len()?];
        rng.fill(&mut raw)?;
        self.import(&raw)
    }

    fn import(&self, raw: &[u8]) -> Result<SymmetricKey, CryptoError> {
        let key = ChaChaPolySymmetricKey::new(self.alg, raw)?;
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

impl ChaChaPolySymmetricState {
    pub fn new(
        alg: SymmetricAlgorithm,
        key: Option<SymmetricKey>,
        options: Option<SymmetricOptions>,
    ) -> Result<Self, CryptoError> {
        let key = key.ok_or(CryptoError::KeyRequired)?;
        let key = key.inner();
        let key = key
            .as_any()
            .downcast_ref::<ChaChaPolySymmetricKey>()
            .ok_or(CryptoError::InvalidKey)?;
        let expected_nonce_len = match alg {
            SymmetricAlgorithm::ChaCha20Poly1305 => 12,
            SymmetricAlgorithm::XChaCha20Poly1305 => 24,
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        };
        let options = options.as_ref().ok_or(CryptoError::NonceRequired)?;
        let nonce = options.locked(|mut options| {
            if options.nonce.is_none() && expected_nonce_len >= 16 {
                options.nonce = Some(vec![0u8; expected_nonce_len]);
                let mut rng = SecureRandom::new();
                rng.fill(options.nonce.as_mut().unwrap())?;
            }
            let nonce_vec = options.nonce.as_ref().ok_or(CryptoError::NonceRequired)?;
            ensure!(
                nonce_vec.len() == expected_nonce_len,
                CryptoError::InvalidNonce
            );
            Ok(nonce_vec.clone())
        })?;
        let aes_gcm_impl = match alg {
            SymmetricAlgorithm::ChaCha20Poly1305 => ChaChaPolyVariant::ChaCha(
                ChaCha20Poly1305::new(GenericArray::from_slice(key.as_raw()?)),
            ),
            SymmetricAlgorithm::XChaCha20Poly1305 => ChaChaPolyVariant::XChaCha(
                XChaCha20Poly1305::new(GenericArray::from_slice(key.as_raw()?)),
            ),
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        };
        let state = ChaChaPolySymmetricState {
            alg,
            options: options.clone(),
            ctx: aes_gcm_impl,
            ad: vec![],
            nonce: Some(nonce),
        };
        Ok(state)
    }
}

impl SymmetricStateLike for ChaChaPolySymmetricState {
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
        let tag = self.encrypt_detached_unchecked(&mut out[..data.len()], data)?;
        out[data.len()..].copy_from_slice(tag.as_ref());
        Ok(out.len())
    }

    fn encrypt_detached_unchecked(
        &mut self,
        out: &mut [u8],
        data: &[u8],
    ) -> Result<SymmetricTag, CryptoError> {
        let nonce = self.nonce.as_ref().ok_or(CryptoError::NonceRequired)?;
        let data_len = data.len();
        if out.len() != data_len {
            bail!(CryptoError::InvalidLength)
        }
        if out.as_ptr() != data.as_ptr() {
            out.copy_from_slice(data);
        }

        let raw_tag = match &self.ctx {
            ChaChaPolyVariant::ChaCha(x) => {
                x.encrypt_in_place_detached(GenericArray::from_slice(&nonce), &self.ad, out)
            }
            ChaChaPolyVariant::XChaCha(x) => {
                x.encrypt_in_place_detached(GenericArray::from_slice(&nonce), &self.ad, out)
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
            ChaChaPolyVariant::ChaCha(x) => x.decrypt_in_place_detached(
                GenericArray::from_slice(&nonce),
                &self.ad,
                out,
                GenericArray::from_slice(raw_tag),
            ),
            ChaChaPolyVariant::XChaCha(x) => x.decrypt_in_place_detached(
                GenericArray::from_slice(&nonce),
                &self.ad,
                out,
                GenericArray::from_slice(raw_tag),
            ),
        }
        .map_err(|_| CryptoError::InvalidTag)?;
        Ok(data.len())
    }
}
