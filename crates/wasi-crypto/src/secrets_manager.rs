use crate::error::*;
use crate::handles::*;
use crate::version::Version;
use crate::CryptoCtx;

#[derive(Clone, Debug)]
pub struct SecretsManager {}

impl CryptoCtx {
    pub fn secrets_manager_open(&self, _options: Option<Handle>) -> Result<Handle, CryptoError> {
        bail!(CryptoError::UnsupportedFeature)
    }

    pub fn secrets_manager_close(
        &self,
        _secrets_manager_handle: Handle,
    ) -> Result<(), CryptoError> {
        bail!(CryptoError::UnsupportedFeature)
    }

    pub fn secrets_manager_invalidate(
        &self,
        _secrets_manager_handle: Handle,
        _key_id: &[u8],
        _key_version: Version,
    ) -> Result<(), CryptoError> {
        bail!(CryptoError::UnsupportedFeature)
    }
}
