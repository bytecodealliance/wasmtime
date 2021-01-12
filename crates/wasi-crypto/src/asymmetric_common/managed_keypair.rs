use crate::error::*;
use crate::handles::*;
use crate::version::Version;
use crate::{AlgorithmType, CryptoCtx};

impl CryptoCtx {
    pub fn keypair_generate_managed(
        &self,
        _secrets_manager_handle: Handle,
        _alg_type: AlgorithmType,
        _alg_str: &str,
        _options_handle: Option<Handle>,
    ) -> Result<Handle, CryptoError> {
        bail!(CryptoError::UnsupportedFeature)
    }

    pub fn keypair_store_managed(
        &self,
        _secrets_manager_handle: Handle,
        kp_handle: Handle,
        _key_id_buf: &mut [u8],
    ) -> Result<(), CryptoError> {
        let _kp = self.handles.symmetric_key.get(kp_handle)?;
        bail!(CryptoError::UnsupportedFeature)
    }

    pub fn keypair_replace_managed(
        &self,
        _secrets_manager_handle: Handle,
        kp_old_handle: Handle,
        kp_new_handle: Handle,
    ) -> Result<Version, CryptoError> {
        let _kp_old = self.handles.keypair.get(kp_old_handle)?;
        let _kp_new = self.handles.keypair.get(kp_new_handle)?;
        bail!(CryptoError::UnsupportedFeature)
    }

    pub fn keypair_from_id(
        &self,
        _secrets_manager_handle: Handle,
        _symmetric_key_id: &[u8],
        _symmetric_key_version: Version,
    ) -> Result<Handle, CryptoError> {
        bail!(CryptoError::UnsupportedFeature)
    }
}
