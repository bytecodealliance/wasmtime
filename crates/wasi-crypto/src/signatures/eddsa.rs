use ed25519_dalek::Signer as _;
use std::sync::Arc;

use super::*;
use crate::asymmetric_common::*;
use crate::error::*;
use crate::rand::SecureRandom;

const KP_LEN: usize = ed25519_dalek::KEYPAIR_LENGTH;
const PK_LEN: usize = ed25519_dalek::PUBLIC_KEY_LENGTH;

#[derive(Debug, Clone)]
pub struct EddsaSignatureSecretKey {
    pub alg: SignatureAlgorithm,
}

#[derive(Debug, Clone)]
pub struct EddsaSignatureKeyPair {
    pub alg: SignatureAlgorithm,
    pub ctx: Arc<ed25519_dalek::Keypair>,
}

impl EddsaSignatureKeyPair {
    fn from_raw(alg: SignatureAlgorithm, raw: &[u8]) -> Result<Self, CryptoError> {
        ensure!(raw.len() == KP_LEN, CryptoError::InvalidKey);
        let ctx = ed25519_dalek::Keypair::from_bytes(raw).map_err(|_| CryptoError::InvalidKey)?;
        Ok(EddsaSignatureKeyPair {
            alg,
            ctx: Arc::new(ctx),
        })
    }

    fn as_raw(&self) -> Result<Vec<u8>, CryptoError> {
        Ok(Vec::from(self.ctx.to_bytes()))
    }

    pub fn generate(
        alg: SignatureAlgorithm,
        _options: Option<SignatureOptions>,
    ) -> Result<Self, CryptoError> {
        let mut rng = SecureRandom::new();
        let ctx = ed25519_dalek::Keypair::generate(&mut rng);
        Ok(EddsaSignatureKeyPair {
            alg,
            ctx: Arc::new(ctx),
        })
    }

    pub fn import(
        alg: SignatureAlgorithm,
        encoded: &[u8],
        encoding: KeyPairEncoding,
    ) -> Result<Self, CryptoError> {
        ensure!(
            alg == SignatureAlgorithm::Ed25519,
            CryptoError::UnsupportedAlgorithm
        );
        let kp = match encoding {
            KeyPairEncoding::Raw => EddsaSignatureKeyPair::from_raw(alg, encoded)?,
            _ => bail!(CryptoError::UnsupportedEncoding),
        };
        Ok(kp)
    }

    pub fn export(&self, encoding: KeyPairEncoding) -> Result<Vec<u8>, CryptoError> {
        match encoding {
            KeyPairEncoding::Raw => self.as_raw(),
            _ => bail!(CryptoError::UnsupportedEncoding),
        }
    }

    pub fn public_key(&self) -> Result<EddsaSignaturePublicKey, CryptoError> {
        let ctx = self.ctx.public;
        Ok(EddsaSignaturePublicKey { alg: self.alg, ctx })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EddsaSignature {
    pub raw: Vec<u8>,
}

impl EddsaSignature {
    pub fn new(raw: Vec<u8>) -> Self {
        EddsaSignature { raw }
    }
    pub fn from_raw(alg: SignatureAlgorithm, raw: &[u8]) -> Result<Self, CryptoError> {
        let expected_len = match alg {
            SignatureAlgorithm::Ed25519 => 64,
            _ => bail!(CryptoError::InvalidSignature),
        };
        ensure!(raw.len() == expected_len, CryptoError::InvalidSignature);
        Ok(Self::new(raw.to_vec()))
    }
}

impl SignatureLike for EddsaSignature {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_ref(&self) -> &[u8] {
        &self.raw
    }
}

#[derive(Debug)]
pub struct EddsaSignatureState {
    pub kp: EddsaSignatureKeyPair,
    pub input: Vec<u8>,
}

impl EddsaSignatureState {
    pub fn new(kp: EddsaSignatureKeyPair) -> Self {
        EddsaSignatureState { kp, input: vec![] }
    }
}

impl SignatureStateLike for EddsaSignatureState {
    fn update(&mut self, input: &[u8]) -> Result<(), CryptoError> {
        self.input.extend_from_slice(input);
        Ok(())
    }

    fn sign(&mut self) -> Result<Signature, CryptoError> {
        let signature_u8 = Vec::from(self.kp.ctx.sign(&self.input).to_bytes());
        let signature = EddsaSignature::new(signature_u8);
        Ok(Signature::new(Box::new(signature)))
    }
}

#[derive(Debug)]
pub struct EddsaSignatureVerificationState {
    pub pk: EddsaSignaturePublicKey,
    pub input: Vec<u8>,
}

impl EddsaSignatureVerificationState {
    pub fn new(pk: EddsaSignaturePublicKey) -> Self {
        EddsaSignatureVerificationState { pk, input: vec![] }
    }
}

impl SignatureVerificationStateLike for EddsaSignatureVerificationState {
    fn update(&mut self, input: &[u8]) -> Result<(), CryptoError> {
        self.input.extend_from_slice(input);
        Ok(())
    }

    fn verify(&self, signature: &Signature) -> Result<(), CryptoError> {
        let signature = signature.inner();
        let signature = signature
            .as_any()
            .downcast_ref::<EddsaSignature>()
            .ok_or(CryptoError::InvalidSignature)?;
        let mut signature_u8 = [0u8; KP_LEN];
        ensure!(
            signature.as_ref().len() == signature_u8.len(),
            CryptoError::InvalidSignature
        );
        signature_u8.copy_from_slice(signature.as_ref());
        let dalek_signature = ed25519_dalek::Signature::new(signature_u8);
        self.pk
            .ctx
            .verify_strict(self.input.as_ref(), &dalek_signature)
            .map_err(|_| CryptoError::VerificationFailed)?;
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct EddsaSignaturePublicKey {
    pub alg: SignatureAlgorithm,
    pub ctx: ed25519_dalek::PublicKey,
}

impl EddsaSignaturePublicKey {
    fn from_raw(alg: SignatureAlgorithm, raw: &[u8]) -> Result<Self, CryptoError> {
        let ctx = ed25519_dalek::PublicKey::from_bytes(raw).map_err(|_| CryptoError::InvalidKey)?;
        let pk = EddsaSignaturePublicKey { alg, ctx };
        Ok(pk)
    }

    fn as_raw(&self) -> Result<Vec<u8>, CryptoError> {
        Ok(Vec::from(self.ctx.to_bytes()))
    }

    pub fn import(
        alg: SignatureAlgorithm,
        encoded: &[u8],
        encoding: PublicKeyEncoding,
    ) -> Result<Self, CryptoError> {
        match encoding {
            PublicKeyEncoding::Raw => Self::from_raw(alg, encoded),
            _ => bail!(CryptoError::UnsupportedEncoding),
        }
    }

    pub fn export(&self, encoding: PublicKeyEncoding) -> Result<Vec<u8>, CryptoError> {
        match encoding {
            PublicKeyEncoding::Raw => self.as_raw(),
            _ => bail!(CryptoError::UnsupportedEncoding),
        }
    }
}
