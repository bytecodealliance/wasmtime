use super::{guest_types, WasiCryptoCtx};

use std::convert::TryInto;
use wasi_crypto::{ensure, CryptoError};

impl super::wasi_ephemeral_crypto_symmetric::WasiEphemeralCryptoSymmetric for WasiCryptoCtx {
    // --- secrets_manager

    fn symmetric_key_generate_managed(
        &self,
        secrets_manager_handle: guest_types::SecretsManager,
        alg_str: &wiggle::GuestPtr<'_, str>,
        options_handle: &guest_types::OptOptions,
    ) -> Result<guest_types::SymmetricKey, guest_types::CryptoErrno> {
        let alg_str = &*alg_str.as_str()?;
        let options_handle = match *options_handle {
            guest_types::OptOptions::Some(options_handle) => Some(options_handle),
            guest_types::OptOptions::None => None,
        };
        Ok(self
            .symmetric_key_generate_managed(
                secrets_manager_handle.into(),
                alg_str,
                options_handle.map(Into::into),
            )?
            .into())
    }

    fn symmetric_key_store_managed(
        &self,
        secrets_manager_handle: guest_types::SecretsManager,
        symmetric_key_handle: guest_types::SymmetricKey,
        symmetric_key_id_ptr: &wiggle::GuestPtr<'_, u8>,
        symmetric_key_id_max_len: guest_types::Size,
    ) -> Result<(), guest_types::CryptoErrno> {
        let key_id_buf = &mut *symmetric_key_id_ptr
            .as_array(symmetric_key_id_max_len)
            .as_slice_mut()?;
        Ok(self.symmetric_key_store_managed(
            secrets_manager_handle.into(),
            symmetric_key_handle.into(),
            key_id_buf,
        )?)
    }

    fn symmetric_key_replace_managed(
        &self,
        secrets_manager_handle: guest_types::SecretsManager,
        symmetric_key_old_handle: guest_types::SymmetricKey,
        symmetric_key_new_handle: guest_types::SymmetricKey,
    ) -> Result<guest_types::Version, guest_types::CryptoErrno> {
        Ok(self
            .symmetric_key_replace_managed(
                secrets_manager_handle.into(),
                symmetric_key_old_handle.into(),
                symmetric_key_new_handle.into(),
            )?
            .into())
    }

    fn symmetric_key_from_id(
        &self,
        secrets_manager_handle: guest_types::SecretsManager,
        symmetric_key_id_ptr: &wiggle::GuestPtr<'_, u8>,
        symmetric_key_id_len: guest_types::Size,
        symmetric_key_version: guest_types::Version,
    ) -> Result<guest_types::SymmetricKey, guest_types::CryptoErrno> {
        let symmetric_key_id = &*symmetric_key_id_ptr
            .as_array(symmetric_key_id_len)
            .as_slice()?;
        Ok(self
            .symmetric_key_from_id(
                secrets_manager_handle.into(),
                symmetric_key_id,
                symmetric_key_version.into(),
            )?
            .into())
    }

    // --- key

    fn symmetric_key_generate(
        &self,
        alg_str: &wiggle::GuestPtr<'_, str>,
        options_handle: &guest_types::OptOptions,
    ) -> Result<guest_types::SymmetricKey, guest_types::CryptoErrno> {
        let alg_str = &*alg_str.as_str()?;
        let options_handle = match *options_handle {
            guest_types::OptOptions::Some(options_handle) => Some(options_handle),
            guest_types::OptOptions::None => None,
        };
        Ok(self
            .symmetric_key_generate(alg_str, options_handle.map(Into::into))?
            .into())
    }

    fn symmetric_key_import(
        &self,
        alg_str: &wiggle::GuestPtr<'_, str>,
        raw_ptr: &wiggle::GuestPtr<'_, u8>,
        raw_len: guest_types::Size,
    ) -> Result<guest_types::SymmetricKey, guest_types::CryptoErrno> {
        let alg_str = &*alg_str.as_str()?;
        let raw = &*raw_ptr.as_array(raw_len).as_slice()?;
        Ok(self.symmetric_key_import(alg_str, raw)?.into())
    }

    fn symmetric_key_export(
        &self,
        symmetric_key_handle: guest_types::SymmetricKey,
    ) -> Result<guest_types::ArrayOutput, guest_types::CryptoErrno> {
        Ok(self
            .symmetric_key_export(symmetric_key_handle.into())?
            .into())
    }

    fn symmetric_key_id(
        &self,
        symmetric_key_handle: guest_types::SymmetricKey,
        symmetric_key_id_ptr: &wiggle::GuestPtr<'_, u8>,
        symmetric_key_id_max_len: guest_types::Size,
    ) -> Result<(guest_types::Size, guest_types::Version), guest_types::CryptoErrno> {
        let key_id_buf = &mut *symmetric_key_id_ptr
            .as_array(symmetric_key_id_max_len)
            .as_slice_mut()?;
        let (key_id, version) = self.symmetric_key_id(symmetric_key_handle.into())?;
        ensure!(
            key_id.len() <= key_id_buf.len(),
            CryptoError::Overflow.into()
        );
        key_id_buf.copy_from_slice(&key_id);
        Ok((key_id.len().try_into()?, version.into()))
    }

    fn symmetric_key_close(
        &self,
        key_handle: guest_types::SymmetricKey,
    ) -> Result<(), guest_types::CryptoErrno> {
        Ok(self.symmetric_key_close(key_handle.into())?)
    }

    // --- state

    fn symmetric_state_open(
        &self,
        alg_str: &wiggle::GuestPtr<'_, str>,
        key_handle: &guest_types::OptSymmetricKey,
        options_handle: &guest_types::OptOptions,
    ) -> Result<guest_types::SymmetricState, guest_types::CryptoErrno> {
        let alg_str = &*alg_str.as_str()?;
        let key_handle = match *key_handle {
            guest_types::OptSymmetricKey::Some(key_handle) => Some(key_handle),
            guest_types::OptSymmetricKey::None => None,
        };
        let options_handle = match *options_handle {
            guest_types::OptOptions::Some(options_handle) => Some(options_handle),
            guest_types::OptOptions::None => None,
        };
        Ok(self
            .symmetric_state_open(
                alg_str,
                key_handle.map(Into::into),
                options_handle.map(Into::into),
            )?
            .into())
    }

    fn symmetric_state_options_get(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
        name_str: &wiggle::GuestPtr<'_, str>,
        value_ptr: &wiggle::GuestPtr<'_, u8>,
        value_max_len: guest_types::Size,
    ) -> Result<guest_types::Size, guest_types::CryptoErrno> {
        let name_str: &str = &*name_str.as_str()?;
        let value = &mut *value_ptr.as_array(value_max_len).as_slice_mut()?;
        Ok(self
            .options_get(symmetric_state_handle.into(), name_str, value)?
            .try_into()?)
    }

    fn symmetric_state_options_get_u64(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
        name_str: &wiggle::GuestPtr<'_, str>,
    ) -> Result<u64, guest_types::CryptoErrno> {
        let name_str: &str = &*name_str.as_str()?;
        Ok(self.options_get_u64(symmetric_state_handle.into(), name_str)?)
    }

    fn symmetric_state_close(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
    ) -> Result<(), guest_types::CryptoErrno> {
        Ok(self.symmetric_state_close(symmetric_state_handle.into())?)
    }

    fn symmetric_state_absorb(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
        data_ptr: &wiggle::GuestPtr<'_, u8>,
        data_len: guest_types::Size,
    ) -> Result<(), guest_types::CryptoErrno> {
        let data = &*data_ptr.as_array(data_len).as_slice()?;
        Ok(self.symmetric_state_absorb(symmetric_state_handle.into(), data)?)
    }

    fn symmetric_state_squeeze(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
        out_ptr: &wiggle::GuestPtr<'_, u8>,
        out_len: guest_types::Size,
    ) -> Result<(), guest_types::CryptoErrno> {
        let out = &mut *out_ptr.as_array(out_len).as_slice_mut()?;
        Ok(self.symmetric_state_squeeze(symmetric_state_handle.into(), out)?)
    }

    fn symmetric_state_squeeze_tag(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
    ) -> Result<guest_types::SymmetricTag, guest_types::CryptoErrno> {
        Ok(self
            .symmetric_state_squeeze_tag(symmetric_state_handle.into())?
            .into())
    }

    fn symmetric_state_squeeze_key(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
        alg_str: &wiggle::GuestPtr<'_, str>,
    ) -> Result<guest_types::SymmetricKey, guest_types::CryptoErrno> {
        let alg_str = &*alg_str.as_str()?;
        Ok(self
            .symmetric_state_squeeze_key(symmetric_state_handle.into(), alg_str)?
            .into())
    }

    fn symmetric_state_max_tag_len(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
    ) -> Result<guest_types::Size, guest_types::CryptoErrno> {
        Ok(self
            .symmetric_state_max_tag_len(symmetric_state_handle.into())?
            .try_into()?)
    }

    fn symmetric_state_encrypt(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
        out_ptr: &wiggle::GuestPtr<'_, u8>,
        out_len: guest_types::Size,
        data_ptr: &wiggle::GuestPtr<'_, u8>,
        data_len: guest_types::Size,
    ) -> Result<guest_types::Size, guest_types::CryptoErrno> {
        let out = &mut *out_ptr.as_array(out_len).as_slice_mut()?;
        let data = &*data_ptr.as_array(data_len).as_slice()?;
        Ok(self
            .symmetric_state_encrypt(symmetric_state_handle.into(), out, data)?
            .try_into()?)
    }

    fn symmetric_state_encrypt_detached(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
        out_ptr: &wiggle::GuestPtr<'_, u8>,
        out_len: guest_types::Size,
        data_ptr: &wiggle::GuestPtr<'_, u8>,
        data_len: guest_types::Size,
    ) -> Result<guest_types::SymmetricTag, guest_types::CryptoErrno> {
        let out = &mut *out_ptr.as_array(out_len).as_slice_mut()?;
        let data = &*data_ptr.as_array(data_len).as_slice()?;
        Ok(self
            .symmetric_state_encrypt_detached(symmetric_state_handle.into(), out, data)?
            .into())
    }

    fn symmetric_state_decrypt(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
        out_ptr: &wiggle::GuestPtr<'_, u8>,
        out_len: guest_types::Size,
        data_ptr: &wiggle::GuestPtr<'_, u8>,
        data_len: guest_types::Size,
    ) -> Result<guest_types::Size, guest_types::CryptoErrno> {
        let out = &mut *out_ptr.as_array(out_len).as_slice_mut()?;
        let data = &*data_ptr.as_array(data_len).as_slice()?;
        Ok(self
            .symmetric_state_decrypt(symmetric_state_handle.into(), out, data)?
            .try_into()?)
    }

    fn symmetric_state_decrypt_detached(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
        out_ptr: &wiggle::GuestPtr<'_, u8>,
        out_len: guest_types::Size,
        data_ptr: &wiggle::GuestPtr<'_, u8>,
        data_len: guest_types::Size,
        raw_tag_ptr: &wiggle::GuestPtr<'_, u8>,
        raw_tag_len: guest_types::Size,
    ) -> Result<guest_types::Size, guest_types::CryptoErrno> {
        let out = &mut *out_ptr.as_array(out_len).as_slice_mut()?;
        let data = &*data_ptr.as_array(data_len).as_slice()?;
        let raw_tag: &[u8] = &*raw_tag_ptr.as_array(raw_tag_len).as_slice()?;
        Ok(self
            .symmetric_state_decrypt_detached(symmetric_state_handle.into(), out, data, raw_tag)?
            .try_into()?)
    }

    fn symmetric_state_ratchet(
        &self,
        symmetric_state_handle: guest_types::SymmetricState,
    ) -> Result<(), guest_types::CryptoErrno> {
        Ok(self.symmetric_state_ratchet(symmetric_state_handle.into())?)
    }

    // --- tag

    fn symmetric_tag_len(
        &self,
        symmetric_tag_handle: guest_types::SymmetricTag,
    ) -> Result<guest_types::Size, guest_types::CryptoErrno> {
        Ok(self
            .symmetric_tag_len(symmetric_tag_handle.into())?
            .try_into()?)
    }

    fn symmetric_tag_pull(
        &self,
        symmetric_tag_handle: guest_types::SymmetricTag,
        buf_ptr: &wiggle::GuestPtr<'_, u8>,
        buf_len: guest_types::Size,
    ) -> Result<guest_types::Size, guest_types::CryptoErrno> {
        let buf = &mut *buf_ptr.as_array(buf_len).as_slice_mut()?;
        Ok(self
            .symmetric_tag_pull(symmetric_tag_handle.into(), buf)?
            .try_into()?)
    }

    fn symmetric_tag_verify(
        &self,
        symmetric_tag_handle: guest_types::SymmetricTag,
        expected_raw_ptr: &wiggle::GuestPtr<'_, u8>,
        expected_raw_len: guest_types::Size,
    ) -> Result<(), guest_types::CryptoErrno> {
        let expected_raw = &*expected_raw_ptr.as_array(expected_raw_len).as_slice()?;
        Ok(self.symmetric_tag_verify(symmetric_tag_handle.into(), expected_raw)?)
    }

    fn symmetric_tag_close(
        &self,
        symmetric_tag_handle: guest_types::SymmetricTag,
    ) -> Result<(), guest_types::CryptoErrno> {
        Ok(self.symmetric_tag_close(symmetric_tag_handle.into())?)
    }
}
