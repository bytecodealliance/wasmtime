use super::*;
use crate::array_output::*;
use crate::version::*;
use crate::CryptoCtx;

use parking_lot::{Mutex, MutexGuard};
use std::sync::Arc;

#[derive(Clone)]
pub struct SymmetricKey {
    inner: Arc<Mutex<Box<dyn SymmetricKeyLike>>>,
}

pub trait SymmetricKeyBuilder {
    fn generate(&self, options: Option<SymmetricOptions>) -> Result<SymmetricKey, CryptoError>;

    fn import(&self, raw: &[u8]) -> Result<SymmetricKey, CryptoError>;

    fn key_len(&self) -> Result<usize, CryptoError>;
}

impl SymmetricKey {
    pub fn new(symmetric_key_like: Box<dyn SymmetricKeyLike>) -> Self {
        SymmetricKey {
            inner: Arc::new(Mutex::new(symmetric_key_like)),
        }
    }

    pub fn inner(&self) -> MutexGuard<'_, Box<dyn SymmetricKeyLike>> {
        self.inner.lock()
    }

    pub fn locked<T, U>(&self, mut f: T) -> U
    where
        T: FnMut(MutexGuard<'_, Box<dyn SymmetricKeyLike>>) -> U,
    {
        f(self.inner())
    }

    pub fn alg(&self) -> SymmetricAlgorithm {
        self.inner().alg()
    }

    pub fn builder(alg_str: &str) -> Result<Box<dyn SymmetricKeyBuilder>, CryptoError> {
        let alg = SymmetricAlgorithm::try_from(alg_str)?;
        let builder = match alg {
            SymmetricAlgorithm::HmacSha256 | SymmetricAlgorithm::HmacSha512 => {
                HmacSha2SymmetricKeyBuilder::new(alg)
            }
            SymmetricAlgorithm::HkdfSha256Expand
            | SymmetricAlgorithm::HkdfSha256Extract
            | SymmetricAlgorithm::HkdfSha512Expand
            | SymmetricAlgorithm::HkdfSha512Extract => HkdfSymmetricKeyBuilder::new(alg),
            SymmetricAlgorithm::Aes128Gcm | SymmetricAlgorithm::Aes256Gcm => {
                AesGcmSymmetricKeyBuilder::new(alg)
            }
            SymmetricAlgorithm::Xoodyak128 | SymmetricAlgorithm::Xoodyak160 => {
                XoodyakSymmetricKeyBuilder::new(alg)
            }
            _ => bail!(CryptoError::InvalidOperation),
        };
        Ok(builder)
    }

    fn generate(
        alg_str: &str,
        options: Option<SymmetricOptions>,
    ) -> Result<SymmetricKey, CryptoError> {
        let builder = Self::builder(alg_str)?;
        builder.generate(options)
    }

    fn import(alg_str: &str, raw: &[u8]) -> Result<SymmetricKey, CryptoError> {
        let builder = Self::builder(alg_str)?;
        builder.import(raw)
    }
}

pub trait SymmetricKeyLike: Sync + Send {
    fn as_any(&self) -> &dyn Any;
    fn alg(&self) -> SymmetricAlgorithm;
    fn as_raw(&self) -> Result<&[u8], CryptoError>;
}

impl CryptoCtx {
    pub fn symmetric_key_generate(
        &self,
        alg_str: &str,
        options_handle: Option<Handle>,
    ) -> Result<Handle, CryptoError> {
        let options = match options_handle {
            None => None,
            Some(options_handle) => {
                Some(self.handles.options.get(options_handle)?.into_symmetric()?)
            }
        };
        let symmetric_key = SymmetricKey::generate(alg_str, options)?;
        let handle = self.handles.symmetric_key.register(symmetric_key)?;
        Ok(handle)
    }

    pub fn symmetric_key_import(&self, alg_str: &str, raw: &[u8]) -> Result<Handle, CryptoError> {
        let symmetric_key = SymmetricKey::import(alg_str, raw)?;
        let handle = self.handles.symmetric_key.register(symmetric_key)?;
        Ok(handle)
    }

    pub fn symmetric_key_export(
        &self,
        symmetric_key_handle: Handle,
    ) -> Result<Handle, CryptoError> {
        let symmetric_key = self.handles.symmetric_key.get(symmetric_key_handle)?;
        let array_output_handle =
            ArrayOutput::register(&self.handles, symmetric_key.inner().as_raw()?.to_vec())?;
        Ok(array_output_handle)
    }

    pub fn symmetric_key_id(
        &self,
        symmetric_key_handle: Handle,
    ) -> Result<(Vec<u8>, Version), CryptoError> {
        let _symmetric_key = self.handles.symmetric_key.get(symmetric_key_handle)?;
        bail!(CryptoError::UnsupportedFeature)
    }

    pub fn symmetric_key_close(&self, symmetric_key_handle: Handle) -> Result<(), CryptoError> {
        self.handles.symmetric_key.close(symmetric_key_handle)
    }
}
