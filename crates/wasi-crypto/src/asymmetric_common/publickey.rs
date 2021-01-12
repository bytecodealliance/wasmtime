use super::*;
use crate::types as guest_types;
use crate::{AlgorithmType, CryptoCtx, HandleManagers};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicKeyEncoding {
    Raw,
    Pkcs8,
    Pem,
    Sec,
    CompressedSec,
    Local,
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

#[derive(Clone)]
pub enum PublicKey {
    Signature(SignaturePublicKey),
    KeyExchange(KxPublicKey),
}

impl PublicKey {
    pub(crate) fn into_signature_public_key(self) -> Result<SignaturePublicKey, CryptoError> {
        match self {
            PublicKey::Signature(pk) => Ok(pk),
            _ => bail!(CryptoError::InvalidHandle),
        }
    }

    pub(crate) fn into_kx_public_key(self) -> Result<KxPublicKey, CryptoError> {
        match self {
            PublicKey::KeyExchange(pk) => Ok(pk),
            _ => bail!(CryptoError::InvalidHandle),
        }
    }

    fn import(
        alg_type: AlgorithmType,
        alg_str: &str,
        encoded: &[u8],
        encoding: PublicKeyEncoding,
    ) -> Result<PublicKey, CryptoError> {
        match alg_type {
            AlgorithmType::Signatures => Ok(PublicKey::Signature(SignaturePublicKey::import(
                SignatureAlgorithm::try_from(alg_str)?,
                encoded,
                encoding,
            )?)),
            _ => bail!(CryptoError::InvalidOperation),
        }
    }

    fn export(&self, encoding: PublicKeyEncoding) -> Result<Vec<u8>, CryptoError> {
        match self {
            PublicKey::Signature(pk) => pk.export(encoding),
            PublicKey::KeyExchange(pk) => pk.export(encoding),
        }
    }

    fn verify(handles: &HandleManagers, pk_handle: Handle) -> Result<(), CryptoError> {
        match handles.publickey.get(pk_handle)? {
            PublicKey::Signature(pk) => SignaturePublicKey::verify(pk),
            PublicKey::KeyExchange(pk) => pk.verify(),
        }
    }
}

impl CryptoCtx {
    pub fn publickey_import(
        &self,
        alg_type: AlgorithmType,
        alg_str: &str,
        encoded: &[u8],
        encoding: PublicKeyEncoding,
    ) -> Result<Handle, CryptoError> {
        let pk = PublicKey::import(alg_type, alg_str, encoded, encoding)?;
        let handle = self.handles.publickey.register(pk)?;
        Ok(handle)
    }

    pub fn publickey_export(
        &self,
        pk_handle: Handle,
        encoding: PublicKeyEncoding,
    ) -> Result<Handle, CryptoError> {
        let pk = self.handles.publickey.get(pk_handle)?;
        let encoded = pk.export(encoding)?;
        let array_output_handle = ArrayOutput::register(&self.handles, encoded)?;
        Ok(array_output_handle)
    }

    pub fn publickey_verify(&self, pk: Handle) -> Result<(), CryptoError> {
        PublicKey::verify(&self.handles, pk)
    }

    pub fn publickey_close(&self, pk: Handle) -> Result<(), CryptoError> {
        self.handles.publickey.close(pk)
    }
}
