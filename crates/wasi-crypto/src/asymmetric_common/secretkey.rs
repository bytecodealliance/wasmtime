use super::*;
use crate::signatures::SignatureSecretKey;
use crate::types as guest_types;
use crate::{AlgorithmType, CryptoCtx};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecretKeyEncoding {
    Raw,
    Pkcs8,
    Pem,
    Sec,
    CompressedSec,
    Local,
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

#[derive(Clone)]
pub enum SecretKey {
    Signature(SignatureSecretKey),
    KeyExchange(KxSecretKey),
}

impl SecretKey {
    pub(crate) fn into_signature_secret_key(self) -> Result<SignatureSecretKey, CryptoError> {
        match self {
            SecretKey::Signature(sk) => Ok(sk),
            _ => bail!(CryptoError::InvalidHandle),
        }
    }

    pub(crate) fn into_kx_secret_key(self) -> Result<KxSecretKey, CryptoError> {
        match self {
            SecretKey::KeyExchange(sk) => Ok(sk),
            _ => bail!(CryptoError::InvalidHandle),
        }
    }

    fn import(
        _alg_type: AlgorithmType,
        _alg_str: &str,
        _encoded: &[u8],
        _encoding: SecretKeyEncoding,
    ) -> Result<SecretKey, CryptoError> {
        bail!(CryptoError::NotImplemented)
    }

    fn export(&self, encoding: SecretKeyEncoding) -> Result<Vec<u8>, CryptoError> {
        match self {
            SecretKey::Signature(sk) => Ok(sk.export(encoding)?),
            SecretKey::KeyExchange(sk) => Ok(sk.export(encoding)?),
        }
    }

    pub fn publickey(&self) -> Result<PublicKey, CryptoError> {
        match self {
            SecretKey::Signature(sk) => Ok(PublicKey::Signature(sk.publickey()?)),
            SecretKey::KeyExchange(sk) => Ok(PublicKey::KeyExchange(sk.publickey()?)),
        }
    }
}

impl CryptoCtx {
    pub fn secretkey_import(
        &self,
        alg_type: AlgorithmType,
        alg_str: &str,
        encoded: &[u8],
        encoding: SecretKeyEncoding,
    ) -> Result<Handle, CryptoError> {
        let sk = SecretKey::import(alg_type, alg_str, encoded, encoding)?;
        let handle = self.handles.secretkey.register(sk)?;
        Ok(handle)
    }

    pub fn secretkey_export(
        &self,
        sk_handle: Handle,
        encoding: SecretKeyEncoding,
    ) -> Result<Handle, CryptoError> {
        let sk = self.handles.secretkey.get(sk_handle)?;
        let encoded = sk.export(encoding)?;
        let array_output_handle = ArrayOutput::register(&self.handles, encoded)?;
        Ok(array_output_handle)
    }

    pub fn publickey(&self, sk_handle: Handle) -> Result<Handle, CryptoError> {
        let sk = self.handles.secretkey.get(sk_handle)?;
        let pk = sk.publickey()?;
        let handle = self.handles.publickey.register(pk)?;
        Ok(handle)
    }

    pub fn secretkey_close(&self, sk: Handle) -> Result<(), CryptoError> {
        self.handles.secretkey.close(sk)
    }
}
