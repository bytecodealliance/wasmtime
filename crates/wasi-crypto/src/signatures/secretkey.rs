use super::ecdsa::*;
use super::eddsa::*;
use super::rsa::*;
use super::*;
use crate::asymmetric_common::SecretKeyEncoding;

#[derive(Clone, Debug)]
pub enum SignatureSecretKey {
    Ecdsa(EcdsaSignatureSecretKey),
    Eddsa(EddsaSignatureSecretKey),
    Rsa(RsaSignatureSecretKey),
}

impl SignatureSecretKey {
    pub fn alg(&self) -> SignatureAlgorithm {
        match self {
            SignatureSecretKey::Ecdsa(x) => x.alg,
            SignatureSecretKey::Eddsa(x) => x.alg,
            SignatureSecretKey::Rsa(x) => x.alg,
        }
    }

    pub(crate) fn export(&self, _encoding: SecretKeyEncoding) -> Result<Vec<u8>, CryptoError> {
        bail!(CryptoError::NotImplemented)
    }

    pub(crate) fn publickey(&self) -> Result<SignaturePublicKey, CryptoError> {
        bail!(CryptoError::NotImplemented)
    }
}
