use super::*;
use crate::asymmetric_common::*;
use crate::CryptoCtx;
use parking_lot::{Mutex, MutexGuard};
use std::sync::Arc;

pub trait KxSecretKeyBuilder {
    fn from_raw(&self, raw: &[u8]) -> Result<KxSecretKey, CryptoError>;
}

#[derive(Clone)]
pub struct KxSecretKey {
    inner: Arc<Mutex<Box<dyn KxSecretKeyLike>>>,
}

impl KxSecretKey {
    pub fn new(kx_secretkey_like: Box<dyn KxSecretKeyLike>) -> Self {
        KxSecretKey {
            inner: Arc::new(Mutex::new(kx_secretkey_like)),
        }
    }

    pub fn inner(&self) -> MutexGuard<'_, Box<dyn KxSecretKeyLike>> {
        self.inner.lock()
    }

    pub fn locked<T, U>(&self, mut f: T) -> U
    where
        T: FnMut(MutexGuard<'_, Box<dyn KxSecretKeyLike>>) -> U,
    {
        f(self.inner())
    }

    pub fn alg(&self) -> KxAlgorithm {
        self.inner().alg()
    }

    pub(crate) fn as_raw(&self) -> Result<Vec<u8>, CryptoError> {
        Ok(self.inner().as_raw()?.to_vec())
    }

    pub(crate) fn export(&self, encoding: SecretKeyEncoding) -> Result<Vec<u8>, CryptoError> {
        match encoding {
            SecretKeyEncoding::Raw => Ok(self.inner().as_raw()?.to_vec()),
            _ => bail!(CryptoError::UnsupportedEncoding),
        }
    }

    pub(crate) fn publickey(&self) -> Result<KxPublicKey, CryptoError> {
        Ok(self.inner().publickey()?)
    }

    pub fn dh(&self, pk: &KxPublicKey) -> Result<Vec<u8>, CryptoError> {
        ensure!(pk.alg() == self.alg(), CryptoError::IncompatibleKeys);
        Ok(self.inner().dh(pk)?)
    }

    fn decapsulate(&self, encapsulated_secret: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Ok(self.inner().decapsulate(encapsulated_secret)?)
    }
}

pub trait KxSecretKeyLike: Sync + Send {
    fn as_any(&self) -> &dyn Any;
    fn alg(&self) -> KxAlgorithm;
    fn len(&self) -> Result<usize, CryptoError>;
    fn as_raw(&self) -> Result<&[u8], CryptoError>;
    fn publickey(&self) -> Result<KxPublicKey, CryptoError>;

    fn dh(&self, _pk: &KxPublicKey) -> Result<Vec<u8>, CryptoError> {
        bail!(CryptoError::InvalidOperation);
    }

    fn decapsulate(&self, _encapsulated_secret: &[u8]) -> Result<Vec<u8>, CryptoError> {
        bail!(CryptoError::InvalidOperation);
    }
}

impl CryptoCtx {
    pub fn kx_dh(&self, pk_handle: Handle, sk_handle: Handle) -> Result<Handle, CryptoError> {
        let pk = self
            .handles
            .publickey
            .get(pk_handle)?
            .into_kx_public_key()?;
        let sk = self
            .handles
            .secretkey
            .get(sk_handle)?
            .into_kx_secret_key()?;
        let shared_secret = sk.dh(&pk)?;
        ArrayOutput::register(&self.handles, shared_secret)
    }

    pub fn kx_decapsulate(
        &self,
        sk_handle: Handle,
        encapsulated_secret: &[u8],
    ) -> Result<Handle, CryptoError> {
        let sk = self
            .handles
            .secretkey
            .get(sk_handle)?
            .into_kx_secret_key()?;
        let shared_secret = sk.decapsulate(encapsulated_secret)?;
        ArrayOutput::register(&self.handles, shared_secret)
    }
}
