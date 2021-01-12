use super::state::*;
use super::*;
use crate::rand::SecureRandom;

use ::xoodyak::*;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

#[derive(Clone, Debug)]
pub struct XoodyakSymmetricState {
    pub alg: SymmetricAlgorithm,
    options: Option<SymmetricOptions>,
    xoodyak_state: XoodyakAny,
}

#[derive(Clone, Debug, Eq)]
pub struct XoodyakSymmetricKey {
    alg: SymmetricAlgorithm,
    raw: Vec<u8>,
}

impl PartialEq for XoodyakSymmetricKey {
    fn eq(&self, other: &Self) -> bool {
        self.alg == other.alg && self.raw.ct_eq(&other.raw).unwrap_u8() == 1
    }
}

impl Drop for XoodyakSymmetricKey {
    fn drop(&mut self) {
        self.raw.zeroize();
    }
}

impl SymmetricKeyLike for XoodyakSymmetricKey {
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

impl XoodyakSymmetricKey {
    fn new(alg: SymmetricAlgorithm, raw: &[u8]) -> Result<Self, CryptoError> {
        Ok(XoodyakSymmetricKey {
            alg,
            raw: raw.to_vec(),
        })
    }
}

pub struct XoodyakSymmetricKeyBuilder {
    alg: SymmetricAlgorithm,
}

impl XoodyakSymmetricKeyBuilder {
    pub fn new(alg: SymmetricAlgorithm) -> Box<dyn SymmetricKeyBuilder> {
        Box::new(Self { alg })
    }
}

impl SymmetricKeyBuilder for XoodyakSymmetricKeyBuilder {
    fn generate(&self, _options: Option<SymmetricOptions>) -> Result<SymmetricKey, CryptoError> {
        let mut rng = SecureRandom::new();
        let mut raw = vec![0u8; self.key_len()?];
        rng.fill(&mut raw)?;
        self.import(&raw)
    }

    fn import(&self, raw: &[u8]) -> Result<SymmetricKey, CryptoError> {
        let key = XoodyakSymmetricKey::new(self.alg, raw)?;
        Ok(SymmetricKey::new(Box::new(key)))
    }

    fn key_len(&self) -> Result<usize, CryptoError> {
        match self.alg {
            SymmetricAlgorithm::Xoodyak128 => Ok(16),
            SymmetricAlgorithm::Xoodyak160 => Ok(24),
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        }
    }
}

impl XoodyakSymmetricState {
    pub fn new(
        alg: SymmetricAlgorithm,
        key: Option<SymmetricKey>,
        options: Option<SymmetricOptions>,
    ) -> Result<Self, CryptoError> {
        let key = match key {
            None => None,
            Some(key) => {
                let key = key.inner();
                let key = key
                    .as_any()
                    .downcast_ref::<XoodyakSymmetricKey>()
                    .ok_or(CryptoError::InvalidKey)?
                    .clone();
                Some(key)
            }
        };
        let nonce = options
            .as_ref()
            .and_then(|options| options.inner.lock().nonce.as_ref().cloned());
        let nonce = nonce.as_deref();
        let xoodyak_state = match key {
            None => XoodyakAny::Hash(XoodyakHash::new()),
            Some(key) => XoodyakAny::Keyed(
                XoodyakKeyed::new(key.as_raw()?, nonce, None, None)
                    .map_err(|_| CryptoError::InvalidKey)?,
            ),
        };
        Ok(XoodyakSymmetricState {
            alg,
            options,
            xoodyak_state,
        })
    }
}

impl SymmetricStateLike for XoodyakSymmetricState {
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
        self.xoodyak_state.absorb(data);
        Ok(())
    }

    fn squeeze(&mut self, out: &mut [u8]) -> Result<(), CryptoError> {
        self.xoodyak_state.squeeze(out);
        Ok(())
    }

    fn squeeze_key(&mut self, alg_str: &str) -> Result<SymmetricKey, CryptoError> {
        let builder = SymmetricKey::builder(alg_str)?;
        let mut raw = vec![0u8; builder.key_len()?];
        self.xoodyak_state.squeeze_key(&mut raw);
        builder.import(&raw)
    }

    fn squeeze_tag(&mut self) -> Result<SymmetricTag, CryptoError> {
        let mut raw_tag = vec![0u8; XOODYAK_AUTH_TAG_BYTES];
        self.xoodyak_state.squeeze(&mut raw_tag);
        let symmetric_tag = SymmetricTag::new(self.alg(), raw_tag);
        Ok(symmetric_tag)
    }

    fn max_tag_len(&mut self) -> Result<usize, CryptoError> {
        Ok(XOODYAK_AUTH_TAG_BYTES)
    }

    fn encrypt_unchecked(&mut self, out: &mut [u8], data: &[u8]) -> Result<usize, CryptoError> {
        let ct_len = data
            .len()
            .checked_add(XOODYAK_AUTH_TAG_BYTES)
            .ok_or(CryptoError::Overflow)?;
        match self.xoodyak_state.aead_encrypt(out, Some(data)) {
            Err(XoodyakError::KeyRequired) => Err(CryptoError::InvalidOperation),
            Err(_) => Err(CryptoError::Overflow),
            Ok(()) => Ok(ct_len),
        }
    }

    fn encrypt_detached_unchecked(
        &mut self,
        out: &mut [u8],
        data: &[u8],
    ) -> Result<SymmetricTag, CryptoError> {
        match self.xoodyak_state.aead_encrypt_detached(out, Some(data)) {
            Err(XoodyakError::KeyRequired) => Err(CryptoError::InvalidOperation),
            Err(_) => Err(CryptoError::Overflow),
            Ok(xoodyak_tag) => {
                let symmetric_tag = SymmetricTag::new(self.alg(), xoodyak_tag.as_ref().to_vec());
                Ok(symmetric_tag)
            }
        }
    }

    fn decrypt_unchecked(&mut self, out: &mut [u8], data: &[u8]) -> Result<usize, CryptoError> {
        let msg_len = data
            .len()
            .checked_sub(XOODYAK_AUTH_TAG_BYTES)
            .ok_or(CryptoError::Overflow)?;
        match self.xoodyak_state.aead_decrypt(out, data) {
            Err(XoodyakError::KeyRequired) => Err(CryptoError::InvalidOperation),
            Err(_) => Err(CryptoError::Overflow),
            Ok(()) => Ok(msg_len),
        }
    }

    fn decrypt_detached_unchecked(
        &mut self,
        out: &mut [u8],
        data: &[u8],
        raw_tag: &[u8],
    ) -> Result<usize, CryptoError> {
        let msg_len = data.len();
        let mut raw_tag_ = [0u8; XOODYAK_AUTH_TAG_BYTES];
        ensure!(raw_tag.len() == raw_tag_.len(), CryptoError::InvalidTag);
        raw_tag_.copy_from_slice(raw_tag);
        match self
            .xoodyak_state
            .aead_decrypt_detached(out, &raw_tag_.into(), Some(data))
        {
            Err(XoodyakError::KeyRequired) => Err(CryptoError::InvalidOperation),
            Err(_) => Err(CryptoError::InvalidTag),
            Ok(()) => Ok(msg_len),
        }
    }

    fn ratchet(&mut self) -> Result<(), CryptoError> {
        self.xoodyak_state
            .ratchet()
            .map_err(|_| CryptoError::InvalidOperation)
    }
}
