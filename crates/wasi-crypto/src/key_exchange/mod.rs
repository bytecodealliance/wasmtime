mod dh;
mod kem;
mod keypair;
mod publickey;
mod secretkey;
mod wasi_glue;

use std::any::Any;
use std::convert::TryFrom;

use self::dh::*;
use self::kem::*;
use crate::array_output::*;
use crate::error::*;
use crate::handles::*;
use crate::options::*;
use parking_lot::Mutex;
use std::sync::Arc;

pub use keypair::*;
pub use publickey::*;
pub use secretkey::*;

#[derive(Debug, Default)]
pub struct KxOptionsInner {
    context: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Default)]
pub struct KxOptions {
    inner: Arc<Mutex<KxOptionsInner>>,
}

impl OptionsLike for KxOptions {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KxAlgorithm {
    X25519,
    Kyber768,
}

impl TryFrom<&str> for KxAlgorithm {
    type Error = CryptoError;

    fn try_from(alg_str: &str) -> Result<Self, CryptoError> {
        match alg_str.to_uppercase().as_str() {
            "X25519" => Ok(KxAlgorithm::X25519),
            "KYBER768" => Ok(KxAlgorithm::Kyber768),
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        }
    }
}

#[test]
fn test_key_exchange() {
    use crate::{AlgorithmType, CryptoCtx, KeyPairEncoding};

    let ctx = CryptoCtx::new();

    let kx_kp_handle1 = ctx
        .keypair_generate(AlgorithmType::KeyExchange, "X25519", None)
        .unwrap();
    let kx_kp_raw_bytes_handle = ctx
        .keypair_export(kx_kp_handle1, KeyPairEncoding::Raw)
        .unwrap();
    let mut kx_kp_raw_bytes = vec![0u8; ctx.array_output_len(kx_kp_raw_bytes_handle).unwrap()];
    ctx.array_output_pull(kx_kp_raw_bytes_handle, &mut kx_kp_raw_bytes)
        .unwrap();

    let pk1 = ctx.keypair_publickey(kx_kp_handle1).unwrap();
    let sk1 = ctx.keypair_secretkey(kx_kp_handle1).unwrap();

    let kx_kp_handle2 = ctx
        .keypair_generate(AlgorithmType::KeyExchange, "X25519", None)
        .unwrap();
    let pk2 = ctx.keypair_publickey(kx_kp_handle2).unwrap();
    let sk2 = ctx.keypair_secretkey(kx_kp_handle2).unwrap();

    let shared_key1_handle = ctx.kx_dh(pk1, sk2).unwrap();
    let mut shared_key1_raw_bytes = vec![0u8; ctx.array_output_len(shared_key1_handle).unwrap()];
    ctx.array_output_pull(shared_key1_handle, &mut shared_key1_raw_bytes)
        .unwrap();

    let shared_key2_handle = ctx.kx_dh(pk2, sk1).unwrap();
    let mut shared_key2_raw_bytes = vec![0u8; ctx.array_output_len(shared_key2_handle).unwrap()];
    ctx.array_output_pull(shared_key2_handle, &mut shared_key2_raw_bytes)
        .unwrap();

    assert_eq!(shared_key1_raw_bytes, shared_key2_raw_bytes);

    ctx.keypair_close(kx_kp_handle1).unwrap();
    ctx.keypair_close(kx_kp_handle2).unwrap();
}

#[cfg(feature = "pqcrypto")]
#[test]
fn test_key_encapsulation() {
    use crate::{AlgorithmType, CryptoCtx, KeyPairEncoding};

    let ctx = CryptoCtx::new();

    let kx_kp_handle = ctx
        .keypair_generate(AlgorithmType::KeyExchange, "Kyber768", None)
        .unwrap();
    let pk = ctx.keypair_publickey(kx_kp_handle).unwrap();
    let sk = ctx.keypair_secretkey(kx_kp_handle).unwrap();

    let (secret_handle, encapsulated_secret_handle) = ctx.kx_encapsulate(pk).unwrap();
    let mut secret_raw_bytes = vec![0u8; ctx.array_output_len(secret_handle).unwrap()];
    ctx.array_output_pull(secret_handle, &mut secret_raw_bytes)
        .unwrap();
    let mut encapsulated_secret_raw_bytes =
        vec![0u8; ctx.array_output_len(encapsulated_secret_handle).unwrap()];
    ctx.array_output_pull(
        encapsulated_secret_handle,
        &mut encapsulated_secret_raw_bytes,
    )
    .unwrap();

    let decapsulated_secret_handle = ctx
        .kx_decapsulate(sk, &encapsulated_secret_raw_bytes)
        .unwrap();
    let mut decapsulated_secret_raw_bytes =
        vec![0u8; ctx.array_output_len(decapsulated_secret_handle).unwrap()];
    ctx.array_output_pull(
        decapsulated_secret_handle,
        &mut decapsulated_secret_raw_bytes,
    )
    .unwrap();

    assert_eq!(secret_raw_bytes, decapsulated_secret_raw_bytes);
}
