use super::*;

use crate::asymmetric_common::*;
use parking_lot::{Mutex, MutexGuard};
use std::sync::Arc;

#[derive(Clone)]
pub struct KxKeyPair {
    inner: Arc<Mutex<Box<dyn KxKeyPairLike>>>,
}

pub trait KxKeyPairBuilder {
    fn generate(&self, options: Option<KxOptions>) -> Result<KxKeyPair, CryptoError>;
}

impl KxKeyPair {
    pub fn new(kx_keypair_like: Box<dyn KxKeyPairLike>) -> Self {
        KxKeyPair {
            inner: Arc::new(Mutex::new(kx_keypair_like)),
        }
    }

    pub fn inner(&self) -> MutexGuard<'_, Box<dyn KxKeyPairLike>> {
        self.inner.lock()
    }

    pub fn locked<T, U>(&self, mut f: T) -> U
    where
        T: FnMut(MutexGuard<'_, Box<dyn KxKeyPairLike>>) -> U,
    {
        f(self.inner())
    }

    pub fn alg(&self) -> KxAlgorithm {
        self.inner().alg()
    }

    pub fn builder(alg: KxAlgorithm) -> Result<Box<dyn KxKeyPairBuilder>, CryptoError> {
        let builder = match alg {
            KxAlgorithm::X25519 => X25519KeyPairBuilder::new(alg),
            #[cfg(feature = "pqcrypto")]
            KxAlgorithm::Kyber768 => Kyber768KeyPairBuilder::new(alg),
            #[cfg(not(feature = "pqcrypto"))]
            KxAlgorithm::Kyber768 => bail!(CryptoError::NotImplemented),
        };
        Ok(builder)
    }

    pub fn generate(
        alg: KxAlgorithm,
        options: Option<KxOptions>,
    ) -> Result<KxKeyPair, CryptoError> {
        let builder = Self::builder(alg)?;
        builder.generate(options)
    }

    pub(crate) fn export(&self, encoding: KeyPairEncoding) -> Result<Vec<u8>, CryptoError> {
        match encoding {
            KeyPairEncoding::Raw => self.inner().as_raw(),
            _ => bail!(CryptoError::UnsupportedEncoding),
        }
    }

    pub(crate) fn public_key(&self) -> Result<KxPublicKey, CryptoError> {
        self.inner().publickey()
    }

    pub(crate) fn secret_key(&self) -> Result<KxSecretKey, CryptoError> {
        self.inner().secretkey()
    }
}

pub trait KxKeyPairLike: Sync + Send {
    fn as_any(&self) -> &dyn Any;
    fn alg(&self) -> KxAlgorithm;
    fn as_raw(&self) -> Result<Vec<u8>, CryptoError> {
        let pk_raw = self.publickey()?.as_raw()?;
        let sk_raw = self.secretkey()?.as_raw()?;
        let mut combined_raw = pk_raw;
        combined_raw.extend_from_slice(&sk_raw);
        Ok(combined_raw)
    }
    fn publickey(&self) -> Result<KxPublicKey, CryptoError>;
    fn secretkey(&self) -> Result<KxSecretKey, CryptoError>;
}
