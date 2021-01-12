use super::ecdsa::*;
use super::eddsa::*;
use super::rsa::*;
use super::*;
use crate::asymmetric_common::*;
use crate::error::*;
#[derive(Clone, Debug)]
pub enum SignaturePublicKey {
    Ecdsa(EcdsaSignaturePublicKey),
    Eddsa(EddsaSignaturePublicKey),
    Rsa(RsaSignaturePublicKey),
}

impl SignaturePublicKey {
    pub fn alg(&self) -> SignatureAlgorithm {
        match self {
            SignaturePublicKey::Ecdsa(x) => x.alg,
            SignaturePublicKey::Eddsa(x) => x.alg,
            SignaturePublicKey::Rsa(x) => x.alg,
        }
    }

    pub(crate) fn import(
        alg: SignatureAlgorithm,
        encoded: &[u8],
        encoding: PublicKeyEncoding,
    ) -> Result<SignaturePublicKey, CryptoError> {
        let pk = match alg.family() {
            SignatureAlgorithmFamily::ECDSA => {
                SignaturePublicKey::Ecdsa(EcdsaSignaturePublicKey::import(alg, encoded, encoding)?)
            }
            SignatureAlgorithmFamily::EdDSA => {
                SignaturePublicKey::Eddsa(EddsaSignaturePublicKey::import(alg, encoded, encoding)?)
            }
            SignatureAlgorithmFamily::RSA => {
                SignaturePublicKey::Rsa(RsaSignaturePublicKey::import(alg, encoded, encoding)?)
            }
        };
        Ok(pk)
    }

    pub(crate) fn export(&self, encoding: PublicKeyEncoding) -> Result<Vec<u8>, CryptoError> {
        let raw_pk = match self {
            SignaturePublicKey::Ecdsa(pk) => pk.export(encoding)?,
            SignaturePublicKey::Eddsa(pk) => pk.export(encoding)?,
            SignaturePublicKey::Rsa(pk) => pk.export(encoding)?,
        };
        Ok(raw_pk)
    }

    pub(crate) fn verify(_pk: SignaturePublicKey) -> Result<(), CryptoError> {
        bail!(CryptoError::NotImplemented)
    }
}
