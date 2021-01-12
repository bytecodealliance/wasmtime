use super::*;
use crate::CryptoCtx;

use parking_lot::{Mutex, MutexGuard};
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Clone)]
pub struct SymmetricState {
    inner: Arc<Mutex<Box<dyn SymmetricStateLike>>>,
}

impl SymmetricState {
    fn new(symmetric_state_like: Box<dyn SymmetricStateLike>) -> Self {
        SymmetricState {
            inner: Arc::new(Mutex::new(symmetric_state_like)),
        }
    }

    fn inner(&self) -> MutexGuard<'_, Box<dyn SymmetricStateLike>> {
        self.inner.lock()
    }

    fn locked<T, U>(&self, mut f: T) -> U
    where
        T: FnMut(MutexGuard<'_, Box<dyn SymmetricStateLike>>) -> U,
    {
        f(self.inner())
    }

    fn open(
        alg_str: &str,
        key: Option<SymmetricKey>,
        options: Option<SymmetricOptions>,
    ) -> Result<SymmetricState, CryptoError> {
        let alg = SymmetricAlgorithm::try_from(alg_str)?;
        if let Some(ref key) = key {
            ensure!(key.alg() == alg, CryptoError::InvalidKey);
        }
        let symmetric_state = match alg {
            SymmetricAlgorithm::HmacSha256 | SymmetricAlgorithm::HmacSha512 => {
                SymmetricState::new(Box::new(HmacSha2SymmetricState::new(alg, key, options)?))
            }
            SymmetricAlgorithm::Sha256
            | SymmetricAlgorithm::Sha512
            | SymmetricAlgorithm::Sha512_256 => {
                SymmetricState::new(Box::new(Sha2SymmetricState::new(alg, None, options)?))
            }
            SymmetricAlgorithm::HkdfSha256Expand
            | SymmetricAlgorithm::HkdfSha512Expand
            | SymmetricAlgorithm::HkdfSha256Extract
            | SymmetricAlgorithm::HkdfSha512Extract => {
                SymmetricState::new(Box::new(HkdfSymmetricState::new(alg, key, options)?))
            }
            SymmetricAlgorithm::Aes128Gcm | SymmetricAlgorithm::Aes256Gcm => {
                SymmetricState::new(Box::new(AesGcmSymmetricState::new(alg, key, options)?))
            }
            SymmetricAlgorithm::Xoodyak128 | SymmetricAlgorithm::Xoodyak160 => {
                SymmetricState::new(Box::new(XoodyakSymmetricState::new(alg, key, options)?))
            }
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        };
        Ok(symmetric_state)
    }
}

pub trait SymmetricStateLike: Sync + Send {
    fn alg(&self) -> SymmetricAlgorithm;
    fn options_get(&self, name: &str) -> Result<Vec<u8>, CryptoError>;
    fn options_get_u64(&self, name: &str) -> Result<u64, CryptoError>;

    fn absorb(&mut self, _data: &[u8]) -> Result<(), CryptoError> {
        bail!(CryptoError::InvalidOperation)
    }

    fn squeeze(&mut self, _out: &mut [u8]) -> Result<(), CryptoError> {
        bail!(CryptoError::InvalidOperation)
    }

    fn squeeze_key(&mut self, _alg_str: &str) -> Result<SymmetricKey, CryptoError> {
        bail!(CryptoError::InvalidOperation)
    }

    fn squeeze_tag(&mut self) -> Result<SymmetricTag, CryptoError> {
        bail!(CryptoError::InvalidOperation)
    }

    fn max_tag_len(&mut self) -> Result<usize, CryptoError> {
        bail!(CryptoError::InvalidOperation)
    }

    fn encrypt_unchecked(&mut self, _out: &mut [u8], _data: &[u8]) -> Result<usize, CryptoError> {
        bail!(CryptoError::InvalidOperation)
    }

    fn encrypt(&mut self, out: &mut [u8], data: &[u8]) -> Result<usize, CryptoError> {
        ensure!(
            out.len()
                == data
                    .len()
                    .checked_add(self.max_tag_len()?)
                    .ok_or(CryptoError::Overflow)?,
            CryptoError::InvalidLength
        );
        self.encrypt_unchecked(out, data)
    }

    fn encrypt_detached_unchecked(
        &mut self,
        _out: &mut [u8],
        _data: &[u8],
    ) -> Result<SymmetricTag, CryptoError> {
        bail!(CryptoError::InvalidOperation)
    }

    fn encrypt_detached(
        &mut self,
        out: &mut [u8],
        data: &[u8],
    ) -> Result<SymmetricTag, CryptoError> {
        ensure!(out.len() == data.len(), CryptoError::InvalidLength);
        self.encrypt_detached_unchecked(out, data)
    }

    fn decrypt_unchecked(&mut self, _out: &mut [u8], _data: &[u8]) -> Result<usize, CryptoError> {
        bail!(CryptoError::InvalidOperation)
    }

    fn decrypt(&mut self, out: &mut [u8], data: &[u8]) -> Result<usize, CryptoError> {
        ensure!(
            out.len()
                == data
                    .len()
                    .checked_sub(self.max_tag_len()?)
                    .ok_or(CryptoError::Overflow)?,
            CryptoError::Overflow
        );
        match self.decrypt_unchecked(out, data) {
            Ok(out_len) => Ok(out_len),
            Err(e) => {
                out.iter_mut().for_each(|x| *x = 0);
                Err(e)
            }
        }
    }

    fn decrypt_detached_unchecked(
        &mut self,
        _out: &mut [u8],
        _data: &[u8],
        _raw_tag: &[u8],
    ) -> Result<usize, CryptoError> {
        bail!(CryptoError::InvalidOperation)
    }

    fn decrypt_detached(
        &mut self,
        out: &mut [u8],
        data: &[u8],
        raw_tag: &[u8],
    ) -> Result<usize, CryptoError> {
        ensure!(out.len() == data.len(), CryptoError::InvalidLength);
        match self.decrypt_detached_unchecked(out, data, raw_tag) {
            Ok(out_len) => Ok(out_len),
            Err(e) => {
                out.iter_mut().for_each(|x| *x = 0);
                Err(e)
            }
        }
    }

    fn ratchet(&mut self) -> Result<(), CryptoError> {
        bail!(CryptoError::InvalidOperation)
    }
}

impl CryptoCtx {
    pub fn symmetric_state_open(
        &self,
        alg_str: &str,
        key_handle: Option<Handle>,
        options_handle: Option<Handle>,
    ) -> Result<Handle, CryptoError> {
        let key = match key_handle {
            None => None,
            Some(symmetric_key_handle) => {
                Some(self.handles.symmetric_key.get(symmetric_key_handle)?)
            }
        };
        let options = match options_handle {
            None => None,
            Some(options_handle) => {
                Some(self.handles.options.get(options_handle)?.into_symmetric()?)
            }
        };
        let symmetric_state = SymmetricState::open(alg_str, key, options)?;
        let handle = self.handles.symmetric_state.register(symmetric_state)?;
        Ok(handle)
    }

    pub fn symmetric_state_options_get(
        &self,
        symmetric_state_handle: Handle,
        name: &str,
        value: &mut [u8],
    ) -> Result<usize, CryptoError> {
        let symmetric_state = self.handles.symmetric_state.get(symmetric_state_handle)?;
        let v = symmetric_state.inner().options_get(name)?;
        let v_len = v.len();
        ensure!(v_len <= value.len(), CryptoError::Overflow);
        value[..v_len].copy_from_slice(&v);
        Ok(v_len)
    }

    pub fn symmetric_state_options_get_u64(
        &self,
        symmetric_state_handle: Handle,
        name: &str,
    ) -> Result<u64, CryptoError> {
        let symmetric_state = self.handles.symmetric_state.get(symmetric_state_handle)?;
        let v = symmetric_state.inner().options_get_u64(name)?;
        Ok(v)
    }

    pub fn symmetric_state_close(&self, symmetric_state_handle: Handle) -> Result<(), CryptoError> {
        self.handles.symmetric_state.close(symmetric_state_handle)
    }

    pub fn symmetric_state_absorb(
        &self,
        symmetric_state_handle: Handle,
        data: &[u8],
    ) -> Result<(), CryptoError> {
        let symmetric_state = self.handles.symmetric_state.get(symmetric_state_handle)?;
        symmetric_state.locked(|mut state| state.absorb(data))
    }

    pub fn symmetric_state_squeeze(
        &self,
        symmetric_state_handle: Handle,
        out: &mut [u8],
    ) -> Result<(), CryptoError> {
        let symmetric_state = self.handles.symmetric_state.get(symmetric_state_handle)?;
        symmetric_state.locked(|mut state| state.squeeze(out))
    }

    pub fn symmetric_state_squeeze_tag(
        &self,
        symmetric_state_handle: Handle,
    ) -> Result<Handle, CryptoError> {
        let symmetric_state = self.handles.symmetric_state.get(symmetric_state_handle)?;
        let tag = symmetric_state.locked(|mut state| state.squeeze_tag())?;
        let handle = self.handles.symmetric_tag.register(tag)?;
        Ok(handle)
    }

    pub fn symmetric_state_squeeze_key(
        &self,
        symmetric_state_handle: Handle,
        alg_str: &str,
    ) -> Result<Handle, CryptoError> {
        let symmetric_state = self.handles.symmetric_state.get(symmetric_state_handle)?;
        let symmetric_key = symmetric_state.locked(|mut state| state.squeeze_key(alg_str))?;
        let handle = self.handles.symmetric_key.register(symmetric_key)?;
        Ok(handle)
    }

    pub fn symmetric_state_max_tag_len(
        &self,
        symmetric_state_handle: Handle,
    ) -> Result<usize, CryptoError> {
        let symmetric_state = self.handles.symmetric_state.get(symmetric_state_handle)?;
        let max_tag_len = symmetric_state.inner().max_tag_len()?;
        Ok(max_tag_len)
    }

    pub fn symmetric_state_encrypt(
        &self,
        symmetric_state_handle: Handle,
        out: &mut [u8],
        data: &[u8],
    ) -> Result<usize, CryptoError> {
        let symmetric_state = self.handles.symmetric_state.get(symmetric_state_handle)?;
        symmetric_state.locked(|mut state| state.encrypt(out, data))
    }

    pub fn symmetric_state_encrypt_detached(
        &self,
        symmetric_state_handle: Handle,
        out: &mut [u8],
        data: &[u8],
    ) -> Result<Handle, CryptoError> {
        let symmetric_state = self.handles.symmetric_state.get(symmetric_state_handle)?;
        let symmetric_tag = symmetric_state.inner().encrypt_detached(out, data)?;
        let handle = self.handles.symmetric_tag.register(symmetric_tag)?;
        Ok(handle)
    }

    pub fn symmetric_state_decrypt(
        &self,
        symmetric_state_handle: Handle,
        out: &mut [u8],
        data: &[u8],
    ) -> Result<usize, CryptoError> {
        let symmetric_state = self.handles.symmetric_state.get(symmetric_state_handle)?;
        symmetric_state.locked(|mut state| state.decrypt(out, data))
    }

    pub fn symmetric_state_decrypt_detached(
        &self,
        symmetric_state_handle: Handle,
        out: &mut [u8],
        data: &[u8],
        raw_tag: &[u8],
    ) -> Result<usize, CryptoError> {
        let symmetric_state = self.handles.symmetric_state.get(symmetric_state_handle)?;
        symmetric_state.locked(|mut state| state.decrypt_detached(out, data, raw_tag))
    }

    pub fn symmetric_state_ratchet(
        &self,
        symmetric_state_handle: Handle,
    ) -> Result<(), CryptoError> {
        let symmetric_state = self.handles.symmetric_state.get(symmetric_state_handle)?;
        symmetric_state.locked(|mut state| state.ratchet())
    }
}
