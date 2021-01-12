use super::*;
use crate::options::Options;
use crate::types as guest_types;
use crate::AlgorithmType;

use std::convert::TryFrom;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyPairEncoding {
    Raw,
    Pkcs8,
    Pem,
    Local,
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

#[derive(Clone)]
pub enum KeyPair {
    Signature(SignatureKeyPair),
    KeyExchange(KxKeyPair),
}

impl KeyPair {
    pub(crate) fn into_signature_keypair(self) -> Result<SignatureKeyPair, CryptoError> {
        match self {
            KeyPair::Signature(kp) => Ok(kp),
            _ => bail!(CryptoError::InvalidHandle),
        }
    }

    pub(crate) fn into_kx_keypair(self) -> Result<KxKeyPair, CryptoError> {
        match self {
            KeyPair::KeyExchange(kp) => Ok(kp),
            _ => bail!(CryptoError::InvalidHandle),
        }
    }

    pub fn export(&self, encoding: KeyPairEncoding) -> Result<Vec<u8>, CryptoError> {
        match self {
            KeyPair::Signature(key_pair) => key_pair.export(encoding),
            KeyPair::KeyExchange(key_pair) => key_pair.export(encoding),
        }
    }

    pub fn generate(
        alg_type: AlgorithmType,
        alg_str: &str,
        options: Option<Options>,
    ) -> Result<KeyPair, CryptoError> {
        match alg_type {
            AlgorithmType::Signatures => {
                let options = match options {
                    None => None,
                    Some(options) => Some(options.into_signatures()?),
                };
                Ok(KeyPair::Signature(SignatureKeyPair::generate(
                    SignatureAlgorithm::try_from(alg_str)?,
                    options,
                )?))
            }
            AlgorithmType::KeyExchange => {
                let options = match options {
                    None => None,
                    Some(options) => Some(options.into_key_exchange()?),
                };
                Ok(KeyPair::KeyExchange(KxKeyPair::generate(
                    KxAlgorithm::try_from(alg_str)?,
                    options,
                )?))
            }
            _ => bail!(CryptoError::InvalidOperation),
        }
    }

    pub fn import(
        alg_type: AlgorithmType,
        alg_str: &str,
        encoded: &[u8],
        encoding: KeyPairEncoding,
    ) -> Result<KeyPair, CryptoError> {
        match alg_type {
            AlgorithmType::Signatures => Ok(KeyPair::Signature(SignatureKeyPair::import(
                SignatureAlgorithm::try_from(alg_str)?,
                encoded,
                encoding,
            )?)),
            _ => bail!(CryptoError::InvalidOperation),
        }
    }

    pub fn from_pk_and_sk(_pk: PublicKey, _sk: SecretKey) -> Result<KeyPair, CryptoError> {
        match (_pk, _sk) {
            (PublicKey::Signature(pk), SecretKey::Signature(sk)) => {
                ensure!(pk.alg() == sk.alg(), CryptoError::IncompatibleKeys);
            }
            (PublicKey::KeyExchange(pk), SecretKey::KeyExchange(sk)) => {
                ensure!(pk.alg() == sk.alg(), CryptoError::IncompatibleKeys);
            }
            _ => bail!(CryptoError::IncompatibleKeys),
        }
        bail!(CryptoError::NotImplemented);
    }

    pub fn public_key(&self) -> Result<PublicKey, CryptoError> {
        match self {
            KeyPair::Signature(key_pair) => Ok(PublicKey::Signature(key_pair.public_key()?)),
            KeyPair::KeyExchange(key_pair) => Ok(PublicKey::KeyExchange(key_pair.public_key()?)),
        }
    }

    pub fn secret_key(&self) -> Result<SecretKey, CryptoError> {
        match self {
            KeyPair::Signature(key_pair) => Ok(SecretKey::Signature(key_pair.secret_key()?)),
            KeyPair::KeyExchange(key_pair) => Ok(SecretKey::KeyExchange(key_pair.secret_key()?)),
        }
    }
}

impl CryptoCtx {
    pub fn keypair_generate(
        &self,
        alg_type: AlgorithmType,
        alg_str: &str,
        options_handle: Option<Handle>,
    ) -> Result<Handle, CryptoError> {
        let options = match options_handle {
            None => None,
            Some(options_handle) => Some(self.handles.options.get(options_handle)?),
        };
        let kp = KeyPair::generate(alg_type, alg_str, options)?;
        let handle = self.handles.keypair.register(kp)?;
        Ok(handle)
    }

    pub fn keypair_import(
        &self,
        alg_type: AlgorithmType,
        alg_str: &str,
        encoded: &[u8],
        encoding: KeyPairEncoding,
    ) -> Result<Handle, CryptoError> {
        let kp = KeyPair::import(alg_type, alg_str, encoded, encoding)?;
        let handle = self.handles.keypair.register(kp)?;
        Ok(handle)
    }

    pub fn keypair_id(&self, kp_handle: Handle) -> Result<(Vec<u8>, Version), CryptoError> {
        let _kp = self.handles.keypair.get(kp_handle)?;
        bail!(CryptoError::UnsupportedFeature)
    }

    pub fn keypair_from_pk_and_sk(
        &self,
        pk_handle: Handle,
        sk_handle: Handle,
    ) -> Result<Handle, CryptoError> {
        let pk = self.handles.publickey.get(pk_handle)?;
        let sk = self.handles.secretkey.get(sk_handle)?;
        let kp = KeyPair::from_pk_and_sk(pk, sk)?;
        let handle = self.handles.keypair.register(kp)?;
        Ok(handle)
    }

    pub fn keypair_export(
        &self,
        kp_handle: Handle,
        encoding: KeyPairEncoding,
    ) -> Result<Handle, CryptoError> {
        let kp = self.handles.keypair.get(kp_handle)?;
        let encoded = kp.export(encoding)?;
        let array_output_handle = ArrayOutput::register(&self.handles, encoded)?;
        Ok(array_output_handle)
    }

    pub fn keypair_publickey(&self, kp_handle: Handle) -> Result<Handle, CryptoError> {
        let kp = self.handles.keypair.get(kp_handle)?;
        let pk = kp.public_key()?;
        let handle = self.handles.publickey.register(pk)?;
        Ok(handle)
    }

    pub fn keypair_secretkey(&self, kp_handle: Handle) -> Result<Handle, CryptoError> {
        let kp = self.handles.keypair.get(kp_handle)?;
        let pk = kp.secret_key()?;
        let handle = self.handles.secretkey.register(pk)?;
        Ok(handle)
    }

    pub fn keypair_close(&self, kp_handle: Handle) -> Result<(), CryptoError> {
        self.handles.keypair.close(kp_handle)
    }
}
