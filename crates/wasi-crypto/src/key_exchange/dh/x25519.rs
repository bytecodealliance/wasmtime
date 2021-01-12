use crate::rand::SecureRandom;

use super::*;
use curve25519_dalek::{
    constants::{BASEPOINT_ORDER, X25519_BASEPOINT},
    montgomery::MontgomeryPoint,
    scalar::Scalar,
};
use subtle::ConstantTimeEq;

const PK_LEN: usize = 32;
const SK_LEN: usize = 32;

#[derive(Clone, Debug)]
pub struct X25519PublicKey {
    alg: KxAlgorithm,
    group_element: MontgomeryPoint,
}

impl X25519PublicKey {
    fn from_group_element(
        alg: KxAlgorithm,
        group_element: MontgomeryPoint,
    ) -> Result<Self, CryptoError> {
        reject_neutral_element(&group_element)?;
        Ok(X25519PublicKey { alg, group_element })
    }

    fn new(alg: KxAlgorithm, raw: &[u8]) -> Result<Self, CryptoError> {
        ensure!(raw.len() == PK_LEN, CryptoError::InvalidKey);
        let mut raw_ = [0u8; PK_LEN];
        raw_.copy_from_slice(&raw);
        let group_element = MontgomeryPoint(raw_);
        X25519PublicKey::from_group_element(alg, group_element)
    }
}

#[derive(Clone, Debug)]
pub struct X25519SecretKey {
    alg: KxAlgorithm,
    raw: Vec<u8>,
    clamped_scalar: Scalar,
}

impl X25519SecretKey {
    fn new(alg: KxAlgorithm, raw: Vec<u8>) -> Result<Self, CryptoError> {
        let mut sk_clamped = [0u8; SK_LEN];
        sk_clamped.copy_from_slice(&raw);
        sk_clamped[0] &= 248;
        sk_clamped[SK_LEN - 1] |= 64;
        let clamped_scalar = Scalar::from_bits(sk_clamped);
        let sk = X25519SecretKey {
            alg,
            raw,
            clamped_scalar,
        };
        Ok(sk)
    }
}

#[derive(Clone, Debug)]
pub struct X25519KeyPair {
    alg: KxAlgorithm,
    pk: X25519PublicKey,
    sk: X25519SecretKey,
}

pub struct X25519KeyPairBuilder {
    alg: KxAlgorithm,
}

impl X25519KeyPairBuilder {
    pub fn new(alg: KxAlgorithm) -> Box<dyn KxKeyPairBuilder> {
        Box::new(Self { alg })
    }
}

fn reject_neutral_element(pk: &MontgomeryPoint) -> Result<(), CryptoError> {
    let zero = [0u8; PK_LEN];
    let mut pk_ = [0u8; PK_LEN];
    pk_.copy_from_slice(&pk.0);
    pk_[PK_LEN - 1] &= 127;
    if zero.ct_eq(pk.as_bytes()).unwrap_u8() == 1 {
        bail!(CryptoError::InvalidKey);
    }
    Ok(())
}

static L: [u8; PK_LEN] = [
    0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x14, 0xde, 0xf9, 0xde, 0xa2, 0xf7, 0x9c, 0xd6, 0x58, 0x12, 0x63, 0x1a, 0x5c, 0xf5, 0xd3, 0xed,
];

fn reject_noncanonical_fe(s: &[u8]) -> Result<(), CryptoError> {
    let mut c: u8 = 0;
    let mut n: u8 = 1;

    let mut i = 31;
    loop {
        c |= ((((s[i] as i32) - (L[i] as i32)) >> 8) as u8) & n;
        n &= ((((s[i] ^ L[i]) as i32) - 1) >> 8) as u8;
        if i == 0 {
            break;
        } else {
            i -= 1;
        }
    }
    if c == 0 {
        Ok(())
    } else {
        bail!(CryptoError::InvalidKey)
    }
}

impl KxKeyPairBuilder for X25519KeyPairBuilder {
    fn generate(&self, _options: Option<KxOptions>) -> Result<KxKeyPair, CryptoError> {
        let mut rng = SecureRandom::new();
        let mut sk_raw = vec![0u8; SK_LEN];
        rng.fill(&mut sk_raw)?;
        let sk = X25519SecretKey::new(self.alg, sk_raw)?;
        let pk = sk.x25519_publickey()?;
        let kp = X25519KeyPair {
            alg: self.alg,
            pk,
            sk,
        };
        Ok(KxKeyPair::new(Box::new(kp)))
    }
}

pub struct X25519SecretKeyBuilder {
    alg: KxAlgorithm,
}

impl KxSecretKeyBuilder for X25519SecretKeyBuilder {
    fn from_raw(&self, raw: &[u8]) -> Result<KxSecretKey, CryptoError> {
        ensure!(raw.len() == SK_LEN, CryptoError::InvalidKey);
        let sk = X25519SecretKey::new(self.alg, raw.to_vec())?;
        Ok(KxSecretKey::new(Box::new(sk)))
    }
}

impl X25519SecretKeyBuilder {
    pub fn new(alg: KxAlgorithm) -> Box<dyn KxSecretKeyBuilder> {
        Box::new(Self { alg })
    }
}

pub struct X25519PublicKeyBuilder {
    alg: KxAlgorithm,
}

impl KxPublicKeyBuilder for X25519PublicKeyBuilder {
    fn from_raw(&self, raw: &[u8]) -> Result<KxPublicKey, CryptoError> {
        ensure!(raw.len() == PK_LEN, CryptoError::InvalidKey);
        let pk = X25519PublicKey::new(self.alg, raw)?;
        Ok(KxPublicKey::new(Box::new(pk)))
    }
}

impl X25519PublicKeyBuilder {
    pub fn new(alg: KxAlgorithm) -> Box<dyn KxPublicKeyBuilder> {
        Box::new(Self { alg })
    }
}

impl KxKeyPairLike for X25519KeyPair {
    fn alg(&self) -> KxAlgorithm {
        self.alg
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn publickey(&self) -> Result<KxPublicKey, CryptoError> {
        Ok(KxPublicKey::new(Box::new(self.pk.clone())))
    }

    fn secretkey(&self) -> Result<KxSecretKey, CryptoError> {
        Ok(KxSecretKey::new(Box::new(self.sk.clone())))
    }
}

impl KxPublicKeyLike for X25519PublicKey {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn alg(&self) -> KxAlgorithm {
        self.alg
    }

    fn len(&self) -> Result<usize, CryptoError> {
        Ok(PK_LEN)
    }

    fn as_raw(&self) -> Result<&[u8], CryptoError> {
        Ok(self.group_element.as_bytes())
    }

    fn verify(&self) -> Result<(), CryptoError> {
        reject_neutral_element(&self.group_element)?;
        reject_noncanonical_fe(&self.group_element.0)?;
        let order_check = BASEPOINT_ORDER * self.group_element;
        ensure!(
            reject_neutral_element(&order_check).is_err(),
            CryptoError::InvalidKey
        );
        Ok(())
    }
}

impl X25519SecretKey {
    fn x25519_publickey(&self) -> Result<X25519PublicKey, CryptoError> {
        let group_element = X25519_BASEPOINT * self.clamped_scalar;
        reject_neutral_element(&group_element).map_err(|_| CryptoError::RNGError)?;
        let pk = X25519PublicKey {
            alg: self.alg,
            group_element,
        };
        Ok(pk)
    }
}

impl KxSecretKeyLike for X25519SecretKey {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn alg(&self) -> KxAlgorithm {
        self.alg
    }

    fn len(&self) -> Result<usize, CryptoError> {
        Ok(SK_LEN)
    }

    fn as_raw(&self) -> Result<&[u8], CryptoError> {
        Ok(&self.raw)
    }

    fn publickey(&self) -> Result<KxPublicKey, CryptoError> {
        Ok(KxPublicKey::new(Box::new(self.x25519_publickey()?)))
    }

    fn dh(&self, pk: &KxPublicKey) -> Result<Vec<u8>, CryptoError> {
        let pk = pk.inner();
        let pk = pk
            .as_any()
            .downcast_ref::<X25519PublicKey>()
            .ok_or(CryptoError::InvalidKey)?;
        let pk_ge: &MontgomeryPoint = &pk.group_element;
        let shared_secret: MontgomeryPoint = pk_ge * self.clamped_scalar;
        reject_neutral_element(&shared_secret)?;
        Ok(shared_secret.as_bytes().to_vec())
    }
}
