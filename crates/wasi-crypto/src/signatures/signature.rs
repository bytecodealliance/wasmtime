use parking_lot::{Mutex, MutexGuard};
use std::convert::TryFrom;
use std::sync::Arc;
use subtle::ConstantTimeEq;

use super::ecdsa::*;
use super::eddsa::*;
use super::keypair::*;
use super::publickey::*;
use super::rsa::*;
use super::*;
use crate::array_output::*;
use crate::error::*;
use crate::handles::*;
use crate::types as guest_types;
use crate::{CryptoCtx, HandleManagers};

#[derive(Clone)]
pub struct Signature {
    inner: Arc<Mutex<Box<dyn SignatureLike>>>,
}

impl PartialEq for Signature {
    fn eq(&self, other: &Self) -> bool {
        let v1 = self.inner();
        let v1 = v1.as_ref();
        let v2 = other.inner();
        let v2 = v2.as_ref();
        v1.as_ref().ct_eq(v2.as_ref()).unwrap_u8() == 1
    }
}

impl Eq for Signature {}

impl Signature {
    pub fn new(signature_like: Box<dyn SignatureLike>) -> Self {
        Signature {
            inner: Arc::new(Mutex::new(signature_like)),
        }
    }

    fn from_raw(alg: SignatureAlgorithm, encoded: &[u8]) -> Result<Self, CryptoError> {
        let signature = match alg.family() {
            SignatureAlgorithmFamily::ECDSA => {
                Signature::new(Box::new(EcdsaSignature::from_raw(alg, encoded)?))
            }
            SignatureAlgorithmFamily::EdDSA => {
                Signature::new(Box::new(EddsaSignature::from_raw(alg, encoded)?))
            }
            SignatureAlgorithmFamily::RSA => {
                Signature::new(Box::new(RsaSignature::from_raw(alg, encoded)?))
            }
        };
        Ok(signature)
    }

    pub fn inner(&self) -> MutexGuard<'_, Box<dyn SignatureLike>> {
        self.inner.lock()
    }

    pub fn locked<T, U>(&self, mut f: T) -> U
    where
        T: FnMut(MutexGuard<'_, Box<dyn SignatureLike>>) -> U,
    {
        f(self.inner())
    }
}

pub trait SignatureLike: Sync + Send {
    fn as_any(&self) -> &dyn Any;
    fn as_ref(&self) -> &[u8];
}

#[derive(Clone)]
pub struct SignatureState {
    inner: Arc<Mutex<Box<dyn SignatureStateLike>>>,
}

impl SignatureState {
    fn new(signature_state_like: Box<dyn SignatureStateLike>) -> Self {
        SignatureState {
            inner: Arc::new(Mutex::new(signature_state_like)),
        }
    }

    fn inner(&self) -> MutexGuard<'_, Box<dyn SignatureStateLike>> {
        self.inner.lock()
    }

    fn locked<T, U>(&self, mut f: T) -> U
    where
        T: FnMut(MutexGuard<'_, Box<dyn SignatureStateLike>>) -> U,
    {
        f(self.inner())
    }

    fn open(handles: &HandleManagers, kp_handle: Handle) -> Result<Handle, CryptoError> {
        let kp = handles.keypair.get(kp_handle)?.into_signature_keypair()?;
        let signature_state = match kp {
            SignatureKeyPair::Ecdsa(kp) => {
                SignatureState::new(Box::new(EcdsaSignatureState::new(kp)))
            }
            SignatureKeyPair::Eddsa(kp) => {
                SignatureState::new(Box::new(EddsaSignatureState::new(kp)))
            }
            SignatureKeyPair::Rsa(kp) => SignatureState::new(Box::new(RsaSignatureState::new(kp))),
        };
        let handle = handles.signature_state.register(signature_state)?;
        Ok(handle)
    }
}

pub trait SignatureStateLike: Sync + Send {
    fn update(&mut self, input: &[u8]) -> Result<(), CryptoError>;
    fn sign(&mut self) -> Result<Signature, CryptoError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignatureEncoding {
    Raw,
    Der,
}

impl From<guest_types::SignatureEncoding> for SignatureEncoding {
    fn from(encoding: guest_types::SignatureEncoding) -> Self {
        match encoding {
            guest_types::SignatureEncoding::Raw => SignatureEncoding::Raw,
            guest_types::SignatureEncoding::Der => SignatureEncoding::Der,
        }
    }
}

#[derive(Clone)]
pub struct SignatureVerificationState {
    inner: Arc<Mutex<Box<dyn SignatureVerificationStateLike>>>,
}

impl SignatureVerificationState {
    fn new(signature_verification_state_like: Box<dyn SignatureVerificationStateLike>) -> Self {
        SignatureVerificationState {
            inner: Arc::new(Mutex::new(signature_verification_state_like)),
        }
    }

    fn inner(&self) -> MutexGuard<'_, Box<dyn SignatureVerificationStateLike>> {
        self.inner.lock()
    }

    fn locked<T, U>(&self, mut f: T) -> U
    where
        T: FnMut(MutexGuard<'_, Box<dyn SignatureVerificationStateLike>>) -> U,
    {
        f(self.inner())
    }

    fn open(handles: &HandleManagers, pk_handle: Handle) -> Result<Handle, CryptoError> {
        let pk = handles
            .publickey
            .get(pk_handle)?
            .into_signature_public_key()?;
        let signature_verification_state = match pk {
            SignaturePublicKey::Ecdsa(pk) => {
                SignatureVerificationState::new(Box::new(EcdsaSignatureVerificationState::new(pk)))
            }
            SignaturePublicKey::Eddsa(pk) => {
                SignatureVerificationState::new(Box::new(EddsaSignatureVerificationState::new(pk)))
            }
            SignaturePublicKey::Rsa(pk) => {
                SignatureVerificationState::new(Box::new(RsaSignatureVerificationState::new(pk)))
            }
        };
        let handle = handles
            .signature_verification_state
            .register(signature_verification_state)?;
        Ok(handle)
    }
}

pub trait SignatureVerificationStateLike: Sync + Send {
    fn update(&mut self, input: &[u8]) -> Result<(), CryptoError>;
    fn verify(&self, signature: &Signature) -> Result<(), CryptoError>;
}

impl CryptoCtx {
    pub fn signature_export(
        &self,
        signature_handle: Handle,
        encoding: SignatureEncoding,
    ) -> Result<Handle, CryptoError> {
        match encoding {
            SignatureEncoding::Raw => {}
            _ => bail!(CryptoError::UnsupportedEncoding),
        }
        let signature = self.handles.signature.get(signature_handle)?;
        let array_output_handle =
            ArrayOutput::register(&self.handles, signature.inner().as_ref().as_ref().to_vec())?;
        Ok(array_output_handle)
    }

    pub fn signature_import(
        &self,
        alg_str: &str,
        encoded: &[u8],
        encoding: SignatureEncoding,
    ) -> Result<Handle, CryptoError> {
        let alg = SignatureAlgorithm::try_from(alg_str)?;
        let signature = match encoding {
            SignatureEncoding::Raw => Signature::from_raw(alg, encoded)?,
            _ => bail!(CryptoError::UnsupportedEncoding),
        };
        let handle = self.handles.signature.register(signature)?;
        Ok(handle)
    }

    pub fn signature_state_open(&self, kp_handle: Handle) -> Result<Handle, CryptoError> {
        SignatureState::open(&self.handles, kp_handle)
    }

    pub fn signature_state_update(
        &self,
        state_handle: Handle,
        input: &[u8],
    ) -> Result<(), CryptoError> {
        let state = self.handles.signature_state.get(state_handle)?;
        state.locked(|mut state| state.update(input))
    }

    pub fn signature_state_sign(&self, state_handle: Handle) -> Result<Handle, CryptoError> {
        let state = self.handles.signature_state.get(state_handle)?;
        let signature = state.locked(|mut state| state.sign())?;
        let handle = self.handles.signature.register(signature)?;
        Ok(handle)
    }

    pub fn signature_state_close(&self, handle: Handle) -> Result<(), CryptoError> {
        self.handles.signature_state.close(handle)
    }

    pub fn signature_verification_state_open(
        &self,
        pk_handle: Handle,
    ) -> Result<Handle, CryptoError> {
        SignatureVerificationState::open(&self.handles, pk_handle)
    }

    pub fn signature_verification_state_update(
        &self,
        verification_state_handle: Handle,
        input: &[u8],
    ) -> Result<(), CryptoError> {
        let state = self
            .handles
            .signature_verification_state
            .get(verification_state_handle)?;
        state.locked(|mut state| state.update(input))
    }

    pub fn signature_verification_state_verify(
        &self,
        verification_state_handle: Handle,
        signature_handle: Handle,
    ) -> Result<(), CryptoError> {
        let state = self
            .handles
            .signature_verification_state
            .get(verification_state_handle)?;
        let signature = self.handles.signature.get(signature_handle)?;
        state.locked(|state| state.verify(&signature))
    }

    pub fn signature_verification_state_close(
        &self,
        verification_state_handle: Handle,
    ) -> Result<(), CryptoError> {
        self.handles
            .signature_verification_state
            .close(verification_state_handle)
    }

    pub fn signature_close(&self, signature_handle: Handle) -> Result<(), CryptoError> {
        self.handles.signature.close(signature_handle)
    }
}
