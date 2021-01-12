use super::ecdsa::*;
use super::eddsa::*;
use super::publickey::*;
use super::rsa::*;
use super::*;
use crate::asymmetric_common::*;
use crate::error::*;

#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum SignatureKeyPair {
    Ecdsa(EcdsaSignatureKeyPair),
    Eddsa(EddsaSignatureKeyPair),
    Rsa(RsaSignatureKeyPair),
}

impl SignatureKeyPair {
    pub(crate) fn export(&self, encoding: KeyPairEncoding) -> Result<Vec<u8>, CryptoError> {
        let encoded = match self {
            SignatureKeyPair::Ecdsa(kp) => kp.export(encoding)?,
            SignatureKeyPair::Eddsa(kp) => kp.export(encoding)?,
            SignatureKeyPair::Rsa(kp) => kp.export(encoding)?,
        };
        Ok(encoded)
    }

    pub(crate) fn generate(
        alg: SignatureAlgorithm,
        options: Option<SignatureOptions>,
    ) -> Result<SignatureKeyPair, CryptoError> {
        let kp = match alg.family() {
            SignatureAlgorithmFamily::ECDSA => {
                SignatureKeyPair::Ecdsa(EcdsaSignatureKeyPair::generate(alg, options)?)
            }
            SignatureAlgorithmFamily::EdDSA => {
                SignatureKeyPair::Eddsa(EddsaSignatureKeyPair::generate(alg, options)?)
            }
            SignatureAlgorithmFamily::RSA => {
                SignatureKeyPair::Rsa(RsaSignatureKeyPair::generate(alg, options)?)
            }
        };
        Ok(kp)
    }

    pub(crate) fn import(
        alg: SignatureAlgorithm,
        encoded: &[u8],
        encoding: KeyPairEncoding,
    ) -> Result<SignatureKeyPair, CryptoError> {
        let kp = match alg.family() {
            SignatureAlgorithmFamily::ECDSA => {
                SignatureKeyPair::Ecdsa(EcdsaSignatureKeyPair::import(alg, encoded, encoding)?)
            }
            SignatureAlgorithmFamily::EdDSA => {
                SignatureKeyPair::Eddsa(EddsaSignatureKeyPair::import(alg, encoded, encoding)?)
            }
            SignatureAlgorithmFamily::RSA => {
                SignatureKeyPair::Rsa(RsaSignatureKeyPair::import(alg, encoded, encoding)?)
            }
        };
        Ok(kp)
    }

    pub(crate) fn public_key(&self) -> Result<SignaturePublicKey, CryptoError> {
        let pk = match self {
            SignatureKeyPair::Ecdsa(kp) => SignaturePublicKey::Ecdsa(kp.public_key()?),
            SignatureKeyPair::Eddsa(kp) => SignaturePublicKey::Eddsa(kp.public_key()?),
            SignatureKeyPair::Rsa(kp) => SignaturePublicKey::Rsa(kp.public_key()?),
        };
        Ok(pk)
    }

    pub(crate) fn secret_key(&self) -> Result<SignatureSecretKey, CryptoError> {
        bail!(CryptoError::NotImplemented)
    }
}
