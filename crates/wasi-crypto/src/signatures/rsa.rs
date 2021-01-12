use ::rsa::{PublicKey as _, PublicKeyParts as _};
use ::sha2::{Digest, Sha256, Sha384, Sha512};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zeroize::Zeroize;

use super::*;
use crate::asymmetric_common::*;
use crate::error::*;
use crate::rand::SecureRandom;

const RAW_ENCODING_VERSION: u16 = 1;
const RAW_ENCODING_ALG_ID: u16 = 1;
const MIN_MODULUS_SIZE: usize = 2048;
const MAX_MODULUS_SIZE: usize = 4096;

#[derive(Debug, Clone)]
pub struct RsaSignatureSecretKey {
    pub alg: SignatureAlgorithm,
}

#[derive(Serialize, Deserialize, Zeroize)]
struct RsaSignatureKeyPairParts {
    version: u16,
    alg_id: u16,
    n: ::rsa::BigUint,
    e: ::rsa::BigUint,
    d: ::rsa::BigUint,
    primes: Vec<::rsa::BigUint>,
}

#[derive(Clone, Debug)]
pub struct RsaSignatureKeyPair {
    pub alg: SignatureAlgorithm,
    ctx: ::rsa::RSAPrivateKey,
}

fn modulus_bits(alg: SignatureAlgorithm) -> Result<usize, CryptoError> {
    let modulus_bits = match alg {
        SignatureAlgorithm::RSA_PKCS1_2048_SHA256
        | SignatureAlgorithm::RSA_PKCS1_2048_SHA384
        | SignatureAlgorithm::RSA_PKCS1_2048_SHA512
        | SignatureAlgorithm::RSA_PSS_2048_SHA256
        | SignatureAlgorithm::RSA_PSS_2048_SHA384
        | SignatureAlgorithm::RSA_PSS_2048_SHA512 => 2048,
        SignatureAlgorithm::RSA_PKCS1_3072_SHA384
        | SignatureAlgorithm::RSA_PKCS1_3072_SHA512
        | SignatureAlgorithm::RSA_PSS_3072_SHA384
        | SignatureAlgorithm::RSA_PSS_3072_SHA512 => 3072,
        SignatureAlgorithm::RSA_PKCS1_4096_SHA512 | SignatureAlgorithm::RSA_PSS_4096_SHA512 => 4096,
        _ => bail!(CryptoError::UnsupportedAlgorithm),
    };
    Ok(modulus_bits)
}

impl RsaSignatureKeyPair {
    fn from_pkcs8(alg: SignatureAlgorithm, pkcs8: &[u8]) -> Result<Self, CryptoError> {
        ensure!(pkcs8.len() < 4096, CryptoError::InvalidKey);
        let ctx = ::rsa::RSAPrivateKey::from_pkcs8(&pkcs8).map_err(|_| CryptoError::InvalidKey)?;
        Ok(RsaSignatureKeyPair { alg, ctx })
    }

    fn from_pem(alg: SignatureAlgorithm, pem: &[u8]) -> Result<Self, CryptoError> {
        ensure!(pem.len() < 4096, CryptoError::InvalidKey);
        let parsed_pem = ::rsa::pem::parse(pem).map_err(|_| CryptoError::InvalidKey)?;
        let ctx =
            ::rsa::RSAPrivateKey::try_from(parsed_pem).map_err(|_| CryptoError::InvalidKey)?;
        Ok(RsaSignatureKeyPair { alg, ctx })
    }

    fn from_local(alg: SignatureAlgorithm, local: &[u8]) -> Result<Self, CryptoError> {
        ensure!(local.len() < 2048, CryptoError::InvalidKey);
        let parts: RsaSignatureKeyPairParts =
            bincode::deserialize(local).map_err(|_| CryptoError::InvalidKey)?;
        ensure!(
            parts.version == RAW_ENCODING_VERSION && parts.alg_id == RAW_ENCODING_ALG_ID,
            CryptoError::InvalidKey
        );
        let ctx = ::rsa::RSAPrivateKey::from_components(parts.n, parts.e, parts.d, parts.primes);
        Ok(RsaSignatureKeyPair { alg, ctx })
    }

    fn to_pkcs8(&self) -> Result<Vec<u8>, CryptoError> {
        let export_key = rsa_export::RsaKey::new(self.ctx.clone());
        export_key
            .as_pkcs8()
            .map_err(|_| CryptoError::InternalError)
    }

    fn to_pem(&self) -> Result<Vec<u8>, CryptoError> {
        let export_key = rsa_export::RsaKey::new(self.ctx.clone());
        export_key
            .as_pkcs8_pem()
            .map(|s| s.as_bytes().to_vec())
            .map_err(|_| CryptoError::InternalError)
    }

    fn to_local(&self) -> Result<Vec<u8>, CryptoError> {
        let parts = RsaSignatureKeyPairParts {
            version: RAW_ENCODING_VERSION,
            alg_id: RAW_ENCODING_ALG_ID,
            n: self.ctx.n().clone(),
            e: self.ctx.e().clone(),
            d: self.ctx.d().clone(),
            primes: self.ctx.primes().to_vec(),
        };
        let local = bincode::serialize(&parts).map_err(|_| CryptoError::InternalError)?;
        Ok(local)
    }

    pub fn generate(
        alg: SignatureAlgorithm,
        _options: Option<SignatureOptions>,
    ) -> Result<Self, CryptoError> {
        let modulus_bits = modulus_bits(alg)?;
        let mut rng = SecureRandom::new();
        let ctx = ::rsa::RSAPrivateKey::new(&mut rng, modulus_bits)
            .map_err(|_| CryptoError::UnsupportedAlgorithm)?;
        Ok(RsaSignatureKeyPair { alg, ctx })
    }

    pub fn import(
        alg: SignatureAlgorithm,
        encoded: &[u8],
        encoding: KeyPairEncoding,
    ) -> Result<Self, CryptoError> {
        match alg.family() {
            SignatureAlgorithmFamily::RSA => {}
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        };
        let mut kp = match encoding {
            KeyPairEncoding::Pkcs8 => Self::from_pkcs8(alg, encoded)?,
            KeyPairEncoding::Pem => Self::from_pem(alg, encoded)?,
            KeyPairEncoding::Local => Self::from_local(alg, encoded)?,
            _ => bail!(CryptoError::UnsupportedEncoding),
        };
        let modulus_size = kp.ctx.size();
        let min_modulus_bits = modulus_bits(alg)?;
        ensure!(
            (min_modulus_bits / 8..=MAX_MODULUS_SIZE / 8).contains(&modulus_size),
            CryptoError::InvalidKey
        );
        kp.ctx.validate().map_err(|_| CryptoError::InvalidKey)?;
        kp.ctx.precompute().map_err(|_| CryptoError::InvalidKey)?;
        Ok(kp)
    }

    pub fn export(&self, encoding: KeyPairEncoding) -> Result<Vec<u8>, CryptoError> {
        match encoding {
            KeyPairEncoding::Pkcs8 => self.to_pkcs8(),
            KeyPairEncoding::Pem => self.to_pem(),
            KeyPairEncoding::Local => self.to_local(),
            _ => bail!(CryptoError::UnsupportedEncoding),
        }
    }

    pub fn public_key(&self) -> Result<RsaSignaturePublicKey, CryptoError> {
        let ctx = self.ctx.to_public_key();
        Ok(RsaSignaturePublicKey { alg: self.alg, ctx })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RsaSignature {
    pub raw: Vec<u8>,
}

impl RsaSignature {
    pub fn new(raw: Vec<u8>) -> Self {
        RsaSignature { raw }
    }

    pub fn from_raw(alg: SignatureAlgorithm, raw: &[u8]) -> Result<Self, CryptoError> {
        let expected_len = modulus_bits(alg)? / 8;
        ensure!(raw.len() == expected_len, CryptoError::InvalidSignature);
        Ok(Self::new(raw.to_vec()))
    }
}

impl SignatureLike for RsaSignature {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_ref(&self) -> &[u8] {
        &self.raw
    }
}

fn padding_scheme(alg: SignatureAlgorithm) -> ::rsa::PaddingScheme {
    match alg {
        SignatureAlgorithm::RSA_PKCS1_2048_SHA256 => {
            ::rsa::PaddingScheme::new_pkcs1v15_sign(Some(::rsa::Hash::SHA2_256))
        }
        SignatureAlgorithm::RSA_PKCS1_2048_SHA384 | SignatureAlgorithm::RSA_PKCS1_3072_SHA384 => {
            ::rsa::PaddingScheme::new_pkcs1v15_sign(Some(::rsa::Hash::SHA2_384))
        }
        SignatureAlgorithm::RSA_PKCS1_2048_SHA512
        | SignatureAlgorithm::RSA_PKCS1_3072_SHA512
        | SignatureAlgorithm::RSA_PKCS1_4096_SHA512 => {
            ::rsa::PaddingScheme::new_pkcs1v15_sign(Some(::rsa::Hash::SHA2_512))
        }

        SignatureAlgorithm::RSA_PSS_2048_SHA256 => {
            ::rsa::PaddingScheme::new_pss::<Sha256, _>(SecureRandom::new())
        }
        SignatureAlgorithm::RSA_PSS_2048_SHA384 | SignatureAlgorithm::RSA_PSS_3072_SHA384 => {
            ::rsa::PaddingScheme::new_pss::<Sha384, _>(SecureRandom::new())
        }
        SignatureAlgorithm::RSA_PSS_2048_SHA512
        | SignatureAlgorithm::RSA_PSS_3072_SHA512
        | SignatureAlgorithm::RSA_PSS_4096_SHA512 => {
            ::rsa::PaddingScheme::new_pss::<Sha512, _>(SecureRandom::new())
        }
        _ => unreachable!(),
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum HashVariant {
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
}

impl HashVariant {
    fn for_alg(alg: SignatureAlgorithm) -> Result<Self, CryptoError> {
        let h = match alg {
            SignatureAlgorithm::RSA_PKCS1_2048_SHA256 | SignatureAlgorithm::RSA_PSS_2048_SHA256 => {
                HashVariant::Sha256(Sha256::new())
            }
            SignatureAlgorithm::RSA_PKCS1_2048_SHA384
            | SignatureAlgorithm::RSA_PKCS1_3072_SHA384
            | SignatureAlgorithm::RSA_PSS_2048_SHA384
            | SignatureAlgorithm::RSA_PSS_3072_SHA384 => HashVariant::Sha384(Sha384::new()),
            SignatureAlgorithm::RSA_PKCS1_2048_SHA512
            | SignatureAlgorithm::RSA_PKCS1_3072_SHA512
            | SignatureAlgorithm::RSA_PKCS1_4096_SHA512
            | SignatureAlgorithm::RSA_PSS_2048_SHA512
            | SignatureAlgorithm::RSA_PSS_3072_SHA512
            | SignatureAlgorithm::RSA_PSS_4096_SHA512 => HashVariant::Sha512(Sha512::new()),
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        };
        Ok(h)
    }
}

#[derive(Debug)]
pub struct RsaSignatureState {
    pub kp: RsaSignatureKeyPair,
    h: HashVariant,
}

impl RsaSignatureState {
    pub fn new(kp: RsaSignatureKeyPair) -> Self {
        let h = HashVariant::for_alg(kp.alg).unwrap();
        RsaSignatureState { kp, h }
    }
}

impl SignatureStateLike for RsaSignatureState {
    fn update(&mut self, input: &[u8]) -> Result<(), CryptoError> {
        match &mut self.h {
            HashVariant::Sha256(x) => x.update(input),
            HashVariant::Sha384(x) => x.update(input),
            HashVariant::Sha512(x) => x.update(input),
        };
        Ok(())
    }

    fn sign(&mut self) -> Result<Signature, CryptoError> {
        let mut rng = SecureRandom::new();
        let digest = match &self.h {
            HashVariant::Sha256(x) => x.clone().finalize().as_slice().to_vec(),
            HashVariant::Sha384(x) => x.clone().finalize().as_slice().to_vec(),
            HashVariant::Sha512(x) => x.clone().finalize().as_slice().to_vec(),
        };
        let encoded_signature = self
            .kp
            .ctx
            .sign_blinded(&mut rng, padding_scheme(self.kp.alg), &digest)
            .map_err(|_| CryptoError::InvalidKey)?;
        let signature = RsaSignature::new(encoded_signature);
        Ok(Signature::new(Box::new(signature)))
    }
}

#[derive(Debug)]
pub struct RsaSignatureVerificationState {
    pub pk: RsaSignaturePublicKey,
    h: HashVariant,
}

impl RsaSignatureVerificationState {
    pub fn new(pk: RsaSignaturePublicKey) -> Self {
        let h = HashVariant::for_alg(pk.alg).unwrap();
        RsaSignatureVerificationState { pk, h }
    }
}

impl SignatureVerificationStateLike for RsaSignatureVerificationState {
    fn update(&mut self, input: &[u8]) -> Result<(), CryptoError> {
        match &mut self.h {
            HashVariant::Sha256(x) => x.update(input),
            HashVariant::Sha384(x) => x.update(input),
            HashVariant::Sha512(x) => x.update(input),
        };
        Ok(())
    }

    fn verify(&self, signature: &Signature) -> Result<(), CryptoError> {
        let signature = signature.inner();
        let signature = signature
            .as_any()
            .downcast_ref::<RsaSignature>()
            .ok_or(CryptoError::InvalidSignature)?;
        let digest = match &self.h {
            HashVariant::Sha256(x) => x.clone().finalize().as_slice().to_vec(),
            HashVariant::Sha384(x) => x.clone().finalize().as_slice().to_vec(),
            HashVariant::Sha512(x) => x.clone().finalize().as_slice().to_vec(),
        };
        self.pk
            .ctx
            .verify(padding_scheme(self.pk.alg), &digest, signature.as_ref())
            .map_err(|_| CryptoError::InvalidSignature)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Zeroize)]
struct RsaSignaturePublicKeyParts {
    version: u16,
    alg_id: u16,
    n: ::rsa::BigUint,
    e: ::rsa::BigUint,
}

#[derive(Clone, Debug)]
pub struct RsaSignaturePublicKey {
    pub alg: SignatureAlgorithm,
    ctx: ::rsa::RSAPublicKey,
}

impl RsaSignaturePublicKey {
    fn from_pkcs8(alg: SignatureAlgorithm, pkcs8: &[u8]) -> Result<Self, CryptoError> {
        ensure!(pkcs8.len() < 4096, CryptoError::InvalidKey);
        let ctx = ::rsa::RSAPublicKey::from_pkcs8(&pkcs8).map_err(|_| CryptoError::InvalidKey)?;
        Ok(RsaSignaturePublicKey { alg, ctx })
    }

    fn from_pem(alg: SignatureAlgorithm, pem: &[u8]) -> Result<Self, CryptoError> {
        ensure!(pem.len() < 4096, CryptoError::InvalidKey);
        let parsed_pem = ::rsa::pem::parse(pem).map_err(|_| CryptoError::InvalidKey)?;
        let ctx = ::rsa::RSAPublicKey::try_from(parsed_pem).map_err(|_| CryptoError::InvalidKey)?;
        Ok(RsaSignaturePublicKey { alg, ctx })
    }

    fn from_local(alg: SignatureAlgorithm, local: &[u8]) -> Result<Self, CryptoError> {
        ensure!(local.len() < 1024, CryptoError::InvalidKey);
        let parts: RsaSignaturePublicKeyParts =
            bincode::deserialize(local).map_err(|_| CryptoError::InvalidKey)?;
        ensure!(
            parts.version == RAW_ENCODING_VERSION && parts.alg_id == RAW_ENCODING_ALG_ID,
            CryptoError::InvalidKey
        );
        let ctx =
            ::rsa::RSAPublicKey::new(parts.n, parts.e).map_err(|_| CryptoError::InvalidKey)?;
        Ok(RsaSignaturePublicKey { alg, ctx })
    }

    fn to_pkcs8(&self) -> Result<Vec<u8>, CryptoError> {
        let export_key = rsa_export::RsaKey::new(self.ctx.clone());
        export_key
            .as_pkcs8()
            .map_err(|_| CryptoError::InternalError)
    }

    fn to_pem(&self) -> Result<Vec<u8>, CryptoError> {
        let export_key = rsa_export::RsaKey::new(self.ctx.clone());
        export_key
            .as_pkcs8_pem()
            .map(|s| s.as_bytes().to_vec())
            .map_err(|_| CryptoError::InternalError)
    }

    fn to_local(&self) -> Result<Vec<u8>, CryptoError> {
        let parts = RsaSignaturePublicKeyParts {
            version: RAW_ENCODING_VERSION,
            alg_id: RAW_ENCODING_ALG_ID,
            n: self.ctx.n().clone(),
            e: self.ctx.e().clone(),
        };
        let local = bincode::serialize(&parts).map_err(|_| CryptoError::InternalError)?;
        Ok(local)
    }

    pub fn import(
        alg: SignatureAlgorithm,
        encoded: &[u8],
        encoding: PublicKeyEncoding,
    ) -> Result<Self, CryptoError> {
        let pk = match encoding {
            PublicKeyEncoding::Pkcs8 => Self::from_pkcs8(alg, encoded)?,
            PublicKeyEncoding::Pem => Self::from_pem(alg, encoded)?,
            PublicKeyEncoding::Local => Self::from_local(alg, encoded)?,
            _ => bail!(CryptoError::UnsupportedEncoding),
        };
        let modulus_size = pk.ctx.size();
        let min_modulus_bits = modulus_bits(alg)?;
        ensure!(
            modulus_size >= min_modulus_bits / 8 && modulus_size <= MAX_MODULUS_SIZE / 8,
            CryptoError::InvalidKey
        );
        Ok(pk)
    }

    pub fn export(&self, encoding: PublicKeyEncoding) -> Result<Vec<u8>, CryptoError> {
        match encoding {
            PublicKeyEncoding::Pkcs8 => self.to_pkcs8(),
            PublicKeyEncoding::Pem => self.to_pem(),
            PublicKeyEncoding::Local => self.to_local(),
            _ => bail!(CryptoError::UnsupportedEncoding),
        }
    }
}
