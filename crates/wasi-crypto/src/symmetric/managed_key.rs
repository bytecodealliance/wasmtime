use crate::error::*;
use crate::handles::*;
use crate::version::Version;
use crate::CryptoCtx;

impl CryptoCtx {
    pub fn symmetric_key_generate_managed(
        &self,
        _secrets_manager_handle: Handle,
        _alg_str: &str,
        _options_handle: Option<Handle>,
    ) -> Result<Handle, CryptoError> {
        bail!(CryptoError::UnsupportedFeature)
    }

    pub fn symmetric_key_store_managed(
        &self,
        _secrets_manager_handle: Handle,
        symmetric_key_handle: Handle,
        _key_id_buf: &mut [u8],
    ) -> Result<(), CryptoError> {
        let _symmetric_key = self.handles.symmetric_key.get(symmetric_key_handle)?;
        bail!(CryptoError::UnsupportedFeature)
    }

    pub fn symmetric_key_replace_managed(
        &self,
        _secrets_manager_handle: Handle,
        symmetric_key_old_handle: Handle,
        symmetric_key_new_handle: Handle,
    ) -> Result<Version, CryptoError> {
        let _symmetric_key_old = self.handles.symmetric_key.get(symmetric_key_old_handle)?;
        let _symmetric_key_new = self.handles.symmetric_key.get(symmetric_key_new_handle)?;
        bail!(CryptoError::UnsupportedFeature)
    }

    pub fn symmetric_key_from_id(
        &self,
        _secrets_manager_handle: Handle,
        _symmetric_key_id: &[u8],
        _symmetric_key_version: Version,
    ) -> Result<Handle, CryptoError> {
        bail!(CryptoError::UnsupportedFeature)
    }
}
