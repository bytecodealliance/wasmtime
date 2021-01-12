mod ecdsa;
mod eddsa;
mod keypair;
mod publickey;
mod rsa;
mod secretkey;
mod signature;
mod wasi_glue;

use crate::options::*;
use crate::{asymmetric_common, error::*};

pub use keypair::*;
pub use publickey::*;
pub use secretkey::*;
pub use signature::*;

use std::any::Any;
use std::convert::TryFrom;

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignatureAlgorithm {
    ECDSA_P256_SHA256,
    ECDSA_K256_SHA256,
    Ed25519,
    RSA_PKCS1_2048_SHA256,
    RSA_PKCS1_2048_SHA384,
    RSA_PKCS1_2048_SHA512,
    RSA_PKCS1_3072_SHA384,
    RSA_PKCS1_3072_SHA512,
    RSA_PKCS1_4096_SHA512,
    RSA_PSS_2048_SHA256,
    RSA_PSS_2048_SHA384,
    RSA_PSS_2048_SHA512,
    RSA_PSS_3072_SHA384,
    RSA_PSS_3072_SHA512,
    RSA_PSS_4096_SHA512,
}

pub enum SignatureAlgorithmFamily {
    ECDSA,
    EdDSA,
    RSA,
}

impl SignatureAlgorithm {
    pub fn family(&self) -> SignatureAlgorithmFamily {
        match self {
            SignatureAlgorithm::ECDSA_P256_SHA256 | SignatureAlgorithm::ECDSA_K256_SHA256 => {
                SignatureAlgorithmFamily::ECDSA
            }
            SignatureAlgorithm::Ed25519 => SignatureAlgorithmFamily::EdDSA,
            SignatureAlgorithm::RSA_PKCS1_2048_SHA256
            | SignatureAlgorithm::RSA_PKCS1_2048_SHA384
            | SignatureAlgorithm::RSA_PKCS1_2048_SHA512
            | SignatureAlgorithm::RSA_PKCS1_3072_SHA384
            | SignatureAlgorithm::RSA_PKCS1_3072_SHA512
            | SignatureAlgorithm::RSA_PKCS1_4096_SHA512
            | SignatureAlgorithm::RSA_PSS_2048_SHA256
            | SignatureAlgorithm::RSA_PSS_2048_SHA384
            | SignatureAlgorithm::RSA_PSS_2048_SHA512
            | SignatureAlgorithm::RSA_PSS_3072_SHA384
            | SignatureAlgorithm::RSA_PSS_3072_SHA512
            | SignatureAlgorithm::RSA_PSS_4096_SHA512 => SignatureAlgorithmFamily::RSA,
        }
    }
}

impl TryFrom<&str> for SignatureAlgorithm {
    type Error = CryptoError;

    fn try_from(alg_str: &str) -> Result<Self, CryptoError> {
        match alg_str.to_uppercase().as_str() {
            "ECDSA_P256_SHA256" => Ok(SignatureAlgorithm::ECDSA_P256_SHA256),
            "ECDSA_K256_SHA256" => Ok(SignatureAlgorithm::ECDSA_K256_SHA256),

            "ED25519" => Ok(SignatureAlgorithm::Ed25519),

            "RSA_PKCS1_2048_SHA256" => Ok(SignatureAlgorithm::RSA_PKCS1_2048_SHA256),
            "RSA_PKCS1_2048_SHA384" => Ok(SignatureAlgorithm::RSA_PKCS1_2048_SHA384),
            "RSA_PKCS1_2048_SHA512" => Ok(SignatureAlgorithm::RSA_PKCS1_2048_SHA512),
            "RSA_PKCS1_3072_SHA384" => Ok(SignatureAlgorithm::RSA_PKCS1_3072_SHA384),
            "RSA_PKCS1_3072_SHA512" => Ok(SignatureAlgorithm::RSA_PKCS1_3072_SHA512),
            "RSA_PKCS1_4096_SHA512" => Ok(SignatureAlgorithm::RSA_PKCS1_4096_SHA512),

            "RSA_PSS_2048_SHA256" => Ok(SignatureAlgorithm::RSA_PSS_2048_SHA256),
            "RSA_PSS_2048_SHA384" => Ok(SignatureAlgorithm::RSA_PSS_2048_SHA384),
            "RSA_PSS_2048_SHA512" => Ok(SignatureAlgorithm::RSA_PSS_2048_SHA512),
            "RSA_PSS_3072_SHA384" => Ok(SignatureAlgorithm::RSA_PSS_3072_SHA384),
            "RSA_PSS_3072_SHA512" => Ok(SignatureAlgorithm::RSA_PSS_3072_SHA512),
            "RSA_PSS_4096_SHA512" => Ok(SignatureAlgorithm::RSA_PSS_4096_SHA512),

            _ => bail!(CryptoError::UnsupportedAlgorithm),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SignatureOptions {}

impl OptionsLike for SignatureOptions {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn set(&mut self, _name: &str, _value: &[u8]) -> Result<(), CryptoError> {
        bail!(CryptoError::UnsupportedOption)
    }

    fn set_u64(&mut self, _name: &str, _value: u64) -> Result<(), CryptoError> {
        bail!(CryptoError::UnsupportedOption)
    }
}

#[test]
fn test_signatures_ecdsa() {
    use crate::{AlgorithmType, CryptoCtx};

    let ctx = CryptoCtx::new();
    let alg = "ECDSA_P256_SHA256";

    let kp_handle = ctx
        .keypair_generate(AlgorithmType::Signatures, alg, None)
        .unwrap();
    let pk_handle = ctx.keypair_publickey(kp_handle).unwrap();

    let pk_serialized = ctx
        .publickey_export(pk_handle, asymmetric_common::PublicKeyEncoding::Raw)
        .unwrap();
    let mut raw = vec![0u8; ctx.array_output_len(pk_serialized).unwrap()];
    ctx.array_output_pull(pk_serialized, &mut raw).unwrap();
    let pk_handle = ctx
        .publickey_import(
            AlgorithmType::Signatures,
            alg,
            &raw,
            asymmetric_common::PublicKeyEncoding::Raw,
        )
        .unwrap();

    let kp_serialized = ctx
        .keypair_export(kp_handle, asymmetric_common::KeyPairEncoding::Raw)
        .unwrap();
    let mut raw = vec![0u8; ctx.array_output_len(kp_serialized).unwrap()];
    ctx.array_output_pull(kp_serialized, &mut raw).unwrap();
    let kp2_handle = ctx
        .keypair_import(
            AlgorithmType::Signatures,
            alg,
            &raw,
            asymmetric_common::KeyPairEncoding::Raw,
        )
        .unwrap();
    let kp_handle = kp2_handle;

    let state_handle = ctx.signature_state_open(kp_handle).unwrap();
    ctx.signature_state_update(state_handle, b"test").unwrap();
    let signature_handle = ctx.signature_state_sign(state_handle).unwrap();

    let verification_state_handle = ctx.signature_verification_state_open(pk_handle).unwrap();
    ctx.signature_verification_state_update(verification_state_handle, b"test")
        .unwrap();
    ctx.signature_verification_state_verify(verification_state_handle, signature_handle)
        .unwrap();

    ctx.signature_verification_state_close(verification_state_handle)
        .unwrap();
    ctx.signature_state_close(state_handle).unwrap();
    ctx.keypair_close(kp_handle).unwrap();
    ctx.publickey_close(pk_handle).unwrap();
    ctx.signature_close(signature_handle).unwrap();
}

#[test]
fn test_signatures_eddsa() {
    use crate::{AlgorithmType, CryptoCtx};

    let ctx = CryptoCtx::new();

    let kp_handle = ctx
        .keypair_generate(AlgorithmType::Signatures, "Ed25519", None)
        .unwrap();
    let pk_handle = ctx.keypair_publickey(kp_handle).unwrap();

    let state_handle = ctx.signature_state_open(kp_handle).unwrap();
    ctx.signature_state_update(state_handle, b"test").unwrap();
    let signature_handle = ctx.signature_state_sign(state_handle).unwrap();

    let verification_state_handle = ctx.signature_verification_state_open(pk_handle).unwrap();
    ctx.signature_verification_state_update(verification_state_handle, b"test")
        .unwrap();
    ctx.signature_verification_state_verify(verification_state_handle, signature_handle)
        .unwrap();

    ctx.signature_verification_state_close(verification_state_handle)
        .unwrap();
    ctx.signature_state_close(state_handle).unwrap();
    ctx.keypair_close(kp_handle).unwrap();
    ctx.publickey_close(pk_handle).unwrap();
    ctx.signature_close(signature_handle).unwrap();
}

#[test]
fn test_signatures_rsa() {
    use crate::{AlgorithmType, CryptoCtx};

    let alg = "RSA_PKCS1_2048_SHA256";
    let ctx = CryptoCtx::new();

    let kp_handle = ctx
        .keypair_generate(AlgorithmType::Signatures, alg, None)
        .unwrap();
    let pk_handle = ctx.keypair_publickey(kp_handle).unwrap();

    let pk_serialized = ctx
        .publickey_export(pk_handle, asymmetric_common::PublicKeyEncoding::Local)
        .unwrap();
    let mut raw = vec![0u8; ctx.array_output_len(pk_serialized).unwrap()];
    ctx.array_output_pull(pk_serialized, &mut raw).unwrap();
    let pk_handle = ctx
        .publickey_import(
            AlgorithmType::Signatures,
            alg,
            &raw,
            asymmetric_common::PublicKeyEncoding::Local,
        )
        .unwrap();

    let kp_serialized = ctx
        .keypair_export(kp_handle, asymmetric_common::KeyPairEncoding::Local)
        .unwrap();
    let mut raw = vec![0u8; ctx.array_output_len(kp_serialized).unwrap()];
    ctx.array_output_pull(kp_serialized, &mut raw).unwrap();
    let kp2_handle = ctx
        .keypair_import(
            AlgorithmType::Signatures,
            alg,
            &raw,
            asymmetric_common::KeyPairEncoding::Local,
        )
        .unwrap();
    let kp_handle = kp2_handle;

    let state_handle = ctx.signature_state_open(kp_handle).unwrap();
    ctx.signature_state_update(state_handle, b"test").unwrap();
    let signature_handle = ctx.signature_state_sign(state_handle).unwrap();

    let verification_state_handle = ctx.signature_verification_state_open(pk_handle).unwrap();
    ctx.signature_verification_state_update(verification_state_handle, b"test")
        .unwrap();
    ctx.signature_verification_state_verify(verification_state_handle, signature_handle)
        .unwrap();

    ctx.signature_verification_state_close(verification_state_handle)
        .unwrap();
    ctx.signature_state_close(state_handle).unwrap();
    ctx.keypair_close(kp_handle).unwrap();
    ctx.publickey_close(pk_handle).unwrap();
    ctx.signature_close(signature_handle).unwrap();
}
