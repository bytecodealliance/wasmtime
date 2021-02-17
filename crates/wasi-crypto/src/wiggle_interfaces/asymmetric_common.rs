use super::{guest_types, WasiCryptoCtx};

use std::convert::TryInto;
use wasi_crypto::{
    ensure, CryptoError, KeyPairEncoding, PublicKeyEncoding, SecretKeyEncoding, Version,
};

impl super::wasi_ephemeral_crypto_asymmetric_common::WasiEphemeralCryptoAsymmetricCommon
    for WasiCryptoCtx
{
    // --- keypair_manager

    fn keypair_generate_managed(
        &self,
        secrets_manager_handle: guest_types::SecretsManager,
        alg_type: guest_types::AlgorithmType,
        alg_str: &wiggle::GuestPtr<'_, str>,
        options_handle: &guest_types::OptOptions,
    ) -> Result<guest_types::Keypair, guest_types::CryptoErrno> {
        let alg_str = &*alg_str.as_str()?;
        let options_handle = match *options_handle {
            guest_types::OptOptions::Some(options_handle) => Some(options_handle),
            guest_types::OptOptions::None => None,
        };
        Ok(self
            .keypair_generate_managed(
                secrets_manager_handle.into(),
                alg_type.into(),
                alg_str,
                options_handle.map(Into::into),
            )?
            .into())
    }

    fn keypair_store_managed(
        &self,
        secrets_manager_handle: guest_types::SecretsManager,
        kp_handle: guest_types::Keypair,
        kp_id_ptr: &wiggle::GuestPtr<'_, u8>,
        kp_id_max_len: guest_types::Size,
    ) -> Result<(), guest_types::CryptoErrno> {
        let key_id_buf = &mut *kp_id_ptr.as_array(kp_id_max_len).as_slice_mut()?;
        Ok(self.keypair_store_managed(
            secrets_manager_handle.into(),
            kp_handle.into(),
            key_id_buf,
        )?)
    }

    fn keypair_replace_managed(
        &self,
        secrets_manager_handle: guest_types::SecretsManager,
        kp_old_handle: guest_types::Keypair,
        kp_new_handle: guest_types::Keypair,
    ) -> Result<guest_types::Version, guest_types::CryptoErrno> {
        Ok(self
            .keypair_replace_managed(
                secrets_manager_handle.into(),
                kp_old_handle.into(),
                kp_new_handle.into(),
            )?
            .0)
    }

    fn keypair_from_id(
        &self,
        secrets_manager_handle: guest_types::SecretsManager,
        kp_id_ptr: &wiggle::GuestPtr<'_, u8>,
        kp_id_len: guest_types::Size,
        kp_version: guest_types::Version,
    ) -> Result<guest_types::Keypair, guest_types::CryptoErrno> {
        let kp_id = &*kp_id_ptr.as_array(kp_id_len).as_slice()?;
        Ok(self
            .keypair_from_id(secrets_manager_handle.into(), kp_id, Version(kp_version))?
            .into())
    }

    // --- keypair

    fn keypair_generate(
        &self,
        alg_type: guest_types::AlgorithmType,
        alg_str: &wiggle::GuestPtr<'_, str>,
        options_handle: &guest_types::OptOptions,
    ) -> Result<guest_types::Keypair, guest_types::CryptoErrno> {
        let alg_str = &*alg_str.as_str()?;
        let options_handle = match *options_handle {
            guest_types::OptOptions::Some(options_handle) => Some(options_handle),
            guest_types::OptOptions::None => None,
        };
        Ok(self
            .keypair_generate(alg_type.into(), alg_str, options_handle.map(Into::into))?
            .into())
    }

    fn keypair_import(
        &self,
        alg_type: guest_types::AlgorithmType,
        alg_str: &wiggle::GuestPtr<'_, str>,
        encoded_ptr: &wiggle::GuestPtr<'_, u8>,
        encoded_len: guest_types::Size,
        encoding: guest_types::KeypairEncoding,
    ) -> Result<guest_types::Keypair, guest_types::CryptoErrno> {
        let alg_str = &*alg_str.as_str()?;
        let encoded = &*encoded_ptr.as_array(encoded_len).as_slice()?;
        Ok(self
            .keypair_import(alg_type.into(), alg_str, encoded, encoding.into())?
            .into())
    }

    fn keypair_id(
        &self,
        kp_handle: guest_types::Keypair,
        kp_id_ptr: &wiggle::GuestPtr<'_, u8>,
        kp_id_max_len: guest_types::Size,
    ) -> Result<(guest_types::Size, guest_types::Version), guest_types::CryptoErrno> {
        let kp_id_buf = &mut *kp_id_ptr.as_array(kp_id_max_len as _).as_slice_mut()?;
        let (kp_id, version) = self.keypair_id(kp_handle.into())?;
        ensure!(kp_id.len() <= kp_id_buf.len(), CryptoError::Overflow.into());
        kp_id_buf.copy_from_slice(&kp_id);
        Ok((kp_id.len().try_into()?, version.0))
    }

    fn keypair_export(
        &self,
        kp_handle: guest_types::Keypair,
        encoding: guest_types::KeypairEncoding,
    ) -> Result<guest_types::ArrayOutput, guest_types::CryptoErrno> {
        Ok(self
            .keypair_export(kp_handle.into(), encoding.into())?
            .into())
    }

    fn keypair_publickey(
        &self,
        kp_handle: guest_types::Keypair,
    ) -> Result<guest_types::Publickey, guest_types::CryptoErrno> {
        Ok(self.keypair_publickey(kp_handle.into())?.into())
    }

    fn keypair_close(
        &self,
        kp_handle: guest_types::Keypair,
    ) -> Result<(), guest_types::CryptoErrno> {
        Ok(self.keypair_close(kp_handle.into())?)
    }

    // --- publickey

    fn publickey_import(
        &self,
        alg_type: guest_types::AlgorithmType,
        alg_str: &wiggle::GuestPtr<'_, str>,
        encoded_ptr: &wiggle::GuestPtr<'_, u8>,
        encoded_len: guest_types::Size,
        encoding: guest_types::PublickeyEncoding,
    ) -> Result<guest_types::Publickey, guest_types::CryptoErrno> {
        let alg_str = &*alg_str.as_str()?;
        let encoded = &*encoded_ptr.as_array(encoded_len).as_slice()?;
        Ok(self
            .publickey_import(alg_type.into(), alg_str, encoded, encoding.into())?
            .into())
    }

    fn publickey_export(
        &self,
        pk_handle: guest_types::Publickey,
        encoding: guest_types::PublickeyEncoding,
    ) -> Result<guest_types::ArrayOutput, guest_types::CryptoErrno> {
        Ok(self
            .publickey_export(pk_handle.into(), encoding.into())?
            .into())
    }

    fn publickey_from_secretkey(
        &self,
        sk_handle: guest_types::Secretkey,
    ) -> Result<guest_types::Publickey, guest_types::CryptoErrno> {
        Ok(self.keypair_publickey(sk_handle.into())?.into())
    }

    fn publickey_verify(
        &self,
        pk_handle: guest_types::Publickey,
    ) -> Result<(), guest_types::CryptoErrno> {
        Ok(self.publickey_verify(pk_handle.into())?)
    }

    fn publickey_close(
        &self,
        pk_handle: guest_types::Publickey,
    ) -> Result<(), guest_types::CryptoErrno> {
        Ok(self.publickey_close(pk_handle.into())?)
    }

    // --- secretkey

    fn secretkey_import(
        &self,
        alg_type: guest_types::AlgorithmType,
        alg_str: &wiggle::GuestPtr<'_, str>,
        encoded_ptr: &wiggle::GuestPtr<'_, u8>,
        encoded_len: guest_types::Size,
        encoding: guest_types::SecretkeyEncoding,
    ) -> Result<guest_types::Secretkey, guest_types::CryptoErrno> {
        let alg_str = &*alg_str.as_str()?;
        let encoded = &*encoded_ptr.as_array(encoded_len).as_slice()?;
        Ok(self
            .secretkey_import(alg_type.into(), alg_str, encoded, encoding.into())?
            .into())
    }

    fn secretkey_export(
        &self,
        sk_handle: guest_types::Secretkey,
        encoding: guest_types::SecretkeyEncoding,
    ) -> Result<guest_types::ArrayOutput, guest_types::CryptoErrno> {
        Ok(self
            .secretkey_export(sk_handle.into(), encoding.into())?
            .into())
    }

    fn secretkey_close(
        &self,
        sk_handle: guest_types::Secretkey,
    ) -> Result<(), guest_types::CryptoErrno> {
        Ok(self.secretkey_close(sk_handle.into())?)
    }

    fn keypair_from_pk_and_sk(
        &self,
        pk_handle: guest_types::Publickey,
        sk_handle: guest_types::Secretkey,
    ) -> Result<guest_types::Keypair, guest_types::CryptoErrno> {
        Ok(self
            .keypair_from_pk_and_sk(pk_handle.into(), sk_handle.into())?
            .into())
    }

    fn keypair_secretkey(
        &self,
        kp_handle: guest_types::Keypair,
    ) -> Result<guest_types::Secretkey, guest_types::CryptoErrno> {
        Ok(self.keypair_secretkey(kp_handle.into())?.into())
    }
}

impl From<guest_types::KeypairEncoding> for KeyPairEncoding {
    fn from(encoding: guest_types::KeypairEncoding) -> Self {
        match encoding {
            guest_types::KeypairEncoding::Raw => KeyPairEncoding::Raw,
            guest_types::KeypairEncoding::Pkcs8 => KeyPairEncoding::Pkcs8,
            guest_types::KeypairEncoding::Pem => KeyPairEncoding::Pem,
            guest_types::KeypairEncoding::Local => KeyPairEncoding::Local,
        }
    }
}

impl From<guest_types::PublickeyEncoding> for PublicKeyEncoding {
    fn from(encoding: guest_types::PublickeyEncoding) -> Self {
        match encoding {
            guest_types::PublickeyEncoding::Raw => PublicKeyEncoding::Raw,
            guest_types::PublickeyEncoding::Pkcs8 => PublicKeyEncoding::Pkcs8,
            guest_types::PublickeyEncoding::Pem => PublicKeyEncoding::Pem,
            guest_types::PublickeyEncoding::Sec => PublicKeyEncoding::Sec,
            guest_types::PublickeyEncoding::CompressedSec => PublicKeyEncoding::CompressedSec,
            guest_types::PublickeyEncoding::Local => PublicKeyEncoding::Local,
        }
    }
}

impl From<guest_types::SecretkeyEncoding> for SecretKeyEncoding {
    fn from(encoding: guest_types::SecretkeyEncoding) -> Self {
        match encoding {
            guest_types::SecretkeyEncoding::Raw => SecretKeyEncoding::Raw,
            guest_types::SecretkeyEncoding::Pkcs8 => SecretKeyEncoding::Pkcs8,
            guest_types::SecretkeyEncoding::Pem => SecretKeyEncoding::Pem,
            guest_types::SecretkeyEncoding::Sec => SecretKeyEncoding::Sec,
            guest_types::SecretkeyEncoding::CompressedSec => SecretKeyEncoding::CompressedSec,
            guest_types::SecretkeyEncoding::Local => SecretKeyEncoding::Local,
        }
    }
}
