mod aes_gcm;
mod chacha_poly;
mod hkdf;
mod hmac_sha2;
mod key;
mod managed_key;
mod sha2;
mod state;
mod tag;
mod wasi_glue;
mod xoodyak;

use self::aes_gcm::*;
use self::hkdf::*;
use self::hmac_sha2::*;
use self::key::*;
use self::sha2::*;
use self::xoodyak::*;
use crate::error::*;
use crate::handles::*;
use crate::options::*;
use parking_lot::{Mutex, MutexGuard};
use std::any::Any;
use std::convert::TryFrom;
use std::sync::Arc;

pub use self::key::SymmetricKey;
pub use self::state::SymmetricState;
pub use self::tag::SymmetricTag;

#[derive(Debug, Default)]
pub struct SymmetricOptionsInner {
    context: Option<Vec<u8>>,
    salt: Option<Vec<u8>>,
    nonce: Option<Vec<u8>>,
    memory_limit: Option<u64>,
    ops_limit: Option<u64>,
    parallelism: Option<u64>,
    guest_buffer: Option<&'static mut [u8]>,
}

#[derive(Clone, Debug)]
pub struct SymmetricOptions {
    inner: Arc<Mutex<SymmetricOptionsInner>>,
}

impl Default for SymmetricOptions {
    fn default() -> Self {
        SymmetricOptions {
            inner: Default::default(),
        }
    }
}

impl SymmetricOptions {
    fn inner(&self) -> MutexGuard<'_, SymmetricOptionsInner> {
        self.inner.lock()
    }

    fn locked<T, U>(&self, mut f: T) -> U
    where
        T: FnMut(MutexGuard<'_, SymmetricOptionsInner>) -> U,
    {
        f(self.inner())
    }
}

impl OptionsLike for SymmetricOptions {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn set_guest_buffer(
        &mut self,
        name: &str,
        guest_buffer: &'static mut [u8],
    ) -> Result<(), CryptoError> {
        let mut inner = self.inner.lock();
        let option = match name.to_lowercase().as_str() {
            "buffer" => &mut inner.guest_buffer,
            _ => bail!(CryptoError::UnsupportedOption),
        };
        *option = Some(guest_buffer);
        Ok(())
    }

    fn set(&mut self, name: &str, value: &[u8]) -> Result<(), CryptoError> {
        let mut inner = self.inner.lock();
        let option = match name.to_lowercase().as_str() {
            "context" => &mut inner.context,
            "salt" => &mut inner.salt,
            "nonce" => &mut inner.nonce,
            _ => bail!(CryptoError::UnsupportedOption),
        };
        *option = Some(value.to_vec());
        Ok(())
    }

    fn get(&self, name: &str) -> Result<Vec<u8>, CryptoError> {
        let inner = self.inner.lock();
        let value = match name.to_lowercase().as_str() {
            "context" => &inner.context,
            "salt" => &inner.salt,
            "nonce" => &inner.nonce,
            _ => bail!(CryptoError::UnsupportedOption),
        };
        value.as_ref().cloned().ok_or(CryptoError::OptionNotSet)
    }

    fn set_u64(&mut self, name: &str, value: u64) -> Result<(), CryptoError> {
        let mut inner = self.inner.lock();
        let option = match name.to_lowercase().as_str() {
            "memory_limit" => &mut inner.memory_limit,
            "ops_limit" => &mut inner.ops_limit,
            "parallelism" => &mut inner.parallelism,
            _ => bail!(CryptoError::UnsupportedOption),
        };
        *option = Some(value);
        Ok(())
    }

    fn get_u64(&self, name: &str) -> Result<u64, CryptoError> {
        let inner = self.inner.lock();
        let value = match name.to_lowercase().as_str() {
            "memory_limit" => &inner.memory_limit,
            "ops_limit" => &inner.ops_limit,
            "parallelism" => &inner.parallelism,
            _ => bail!(CryptoError::UnsupportedOption),
        };
        value.ok_or(CryptoError::OptionNotSet)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymmetricAlgorithm {
    None,
    HmacSha256,
    HmacSha512,
    HkdfSha256Extract,
    HkdfSha512Extract,
    HkdfSha256Expand,
    HkdfSha512Expand,
    Sha256,
    Sha512,
    Sha512_256,
    Aes128Gcm,
    Aes256Gcm,
    ChaCha20Poly1305,
    XChaCha20Poly1305,
    Xoodyak128,
    Xoodyak160,
}

impl TryFrom<&str> for SymmetricAlgorithm {
    type Error = CryptoError;

    fn try_from(alg_str: &str) -> Result<Self, CryptoError> {
        match alg_str.to_uppercase().as_str() {
            "HKDF-EXTRACT/SHA-256" => Ok(SymmetricAlgorithm::HkdfSha256Extract),
            "HKDF-EXTRACT/SHA-512" => Ok(SymmetricAlgorithm::HkdfSha512Extract),
            "HKDF-EXPAND/SHA-256" => Ok(SymmetricAlgorithm::HkdfSha256Expand),
            "HKDF-EXPAND/SHA-512" => Ok(SymmetricAlgorithm::HkdfSha512Expand),
            "HMAC/SHA-256" => Ok(SymmetricAlgorithm::HmacSha256),
            "HMAC/SHA-512" => Ok(SymmetricAlgorithm::HmacSha512),
            "SHA-256" => Ok(SymmetricAlgorithm::Sha256),
            "SHA-512" => Ok(SymmetricAlgorithm::Sha512),
            "SHA-512/256" => Ok(SymmetricAlgorithm::Sha512_256),
            "AES-128-GCM" => Ok(SymmetricAlgorithm::Aes128Gcm),
            "AES-256-GCM" => Ok(SymmetricAlgorithm::Aes256Gcm),
            "CHACHA20-POLY1305" => Ok(SymmetricAlgorithm::ChaCha20Poly1305),
            "XCHACHA20-POLY1305" => Ok(SymmetricAlgorithm::XChaCha20Poly1305),
            "XOODYAK-128" => Ok(SymmetricAlgorithm::Xoodyak128),
            "XOODYAK-160" => Ok(SymmetricAlgorithm::Xoodyak160),
            _ => bail!(CryptoError::UnsupportedAlgorithm),
        }
    }
}

#[test]
fn test_hash() {
    use crate::CryptoCtx;

    let ctx = CryptoCtx::new();

    let state_handle = ctx.symmetric_state_open("SHA-256", None, None).unwrap();
    ctx.symmetric_state_absorb(state_handle, b"data").unwrap();
    ctx.symmetric_state_absorb(state_handle, b"more_data")
        .unwrap();
    let mut out = [0u8; 32];
    ctx.symmetric_state_squeeze(state_handle, &mut out).unwrap();
    let expected = [
        19, 196, 14, 236, 34, 84, 26, 21, 94, 23, 32, 16, 199, 253, 110, 246, 84, 228, 225, 56,
        160, 194, 9, 35, 249, 169, 16, 98, 162, 127, 87, 182,
    ];
    assert_eq!(out, expected);
    ctx.symmetric_state_close(state_handle).unwrap();
}

#[test]
fn test_hmac() {
    use crate::CryptoCtx;

    let ctx = CryptoCtx::new();

    let key_handle = ctx.symmetric_key_generate("HMAC/SHA-512", None).unwrap();
    let state_handle = ctx
        .symmetric_state_open("HMAC/SHA-512", Some(key_handle), None)
        .unwrap();
    ctx.symmetric_state_absorb(state_handle, b"data").unwrap();
    ctx.symmetric_state_absorb(state_handle, b"more_data")
        .unwrap();

    let tag_handle = ctx.symmetric_state_squeeze_tag(state_handle).unwrap();
    let raw_tag = tag_to_vec(&ctx, tag_handle).unwrap();

    let tag_handle = ctx.symmetric_state_squeeze_tag(state_handle).unwrap();
    ctx.symmetric_tag_verify(tag_handle, &raw_tag).unwrap();

    ctx.symmetric_state_close(state_handle).unwrap();
    ctx.symmetric_key_close(key_handle).unwrap();
    ctx.symmetric_tag_close(tag_handle).unwrap();
}

#[test]
fn test_hkdf() {
    use crate::CryptoCtx;

    let ctx = CryptoCtx::new();

    let mut out = vec![0u8; 32];

    let key_handle = ctx
        .symmetric_key_import("HKDF-EXTRACT/SHA-512", b"IKM")
        .unwrap();
    let state_handle = ctx
        .symmetric_state_open("HKDF-EXTRACT/SHA-512", Some(key_handle), None)
        .unwrap();
    ctx.symmetric_state_absorb(state_handle, b"salt").unwrap();
    let prk_handle = ctx
        .symmetric_state_squeeze_key(state_handle, "HKDF-EXPAND/SHA-512")
        .unwrap();
    ctx.symmetric_state_close(state_handle).unwrap();
    ctx.symmetric_key_close(key_handle).unwrap();

    let state_handle = ctx
        .symmetric_state_open("HKDF-EXPAND/SHA-512", Some(prk_handle), None)
        .unwrap();
    ctx.symmetric_state_absorb(state_handle, b"info").unwrap();
    ctx.symmetric_state_squeeze(state_handle, &mut out).unwrap();

    ctx.symmetric_state_close(state_handle).unwrap();
}

#[test]
fn test_encryption() {
    use crate::{AlgorithmType, CryptoCtx};

    let ctx = CryptoCtx::new();

    let msg = b"test";
    let nonce = [42u8; 12];
    let key_handle = ctx.symmetric_key_generate("AES-256-GCM", None).unwrap();

    let options_handle = ctx.options_open(AlgorithmType::Symmetric).unwrap();
    ctx.options_set(options_handle, "nonce", &nonce).unwrap();

    let symmetric_state = ctx
        .symmetric_state_open("AES-256-GCM", Some(key_handle), Some(options_handle))
        .unwrap();
    let mut observed_nonce = [0u8; 12];
    ctx.symmetric_state_options_get(symmetric_state, "nonce", &mut observed_nonce)
        .unwrap();
    assert_eq!(&nonce, &observed_nonce);

    let mut ciphertext_with_tag =
        vec![0u8; msg.len() + ctx.symmetric_state_max_tag_len(symmetric_state).unwrap()];
    ctx.symmetric_state_encrypt(symmetric_state, &mut ciphertext_with_tag, msg)
        .unwrap();
    ctx.symmetric_state_close(symmetric_state).unwrap();

    let symmetric_state = ctx
        .symmetric_state_open("AES-256-GCM", Some(key_handle), Some(options_handle))
        .unwrap();
    let mut msg2 = vec![0u8; msg.len()];
    ctx.symmetric_state_decrypt(symmetric_state, &mut msg2, &ciphertext_with_tag)
        .unwrap();
    ctx.symmetric_state_close(symmetric_state).unwrap();
    assert_eq!(msg, &msg2[..]);

    let symmetric_state = ctx
        .symmetric_state_open("AES-256-GCM", Some(key_handle), Some(options_handle))
        .unwrap();
    let mut ciphertext = vec![0u8; msg.len()];
    let tag_handle = ctx
        .symmetric_state_encrypt_detached(symmetric_state, &mut ciphertext, msg)
        .unwrap();
    ctx.symmetric_state_close(symmetric_state).unwrap();

    let raw_tag = tag_to_vec(&ctx, tag_handle).unwrap();

    let symmetric_state = ctx
        .symmetric_state_open("AES-256-GCM", Some(key_handle), Some(options_handle))
        .unwrap();
    let mut msg2 = vec![0u8; msg.len()];
    ctx.symmetric_state_decrypt_detached(symmetric_state, &mut msg2, &ciphertext, &raw_tag)
        .unwrap();
    ctx.symmetric_state_close(symmetric_state).unwrap();
    assert_eq!(msg, &msg2[..]);
}

#[cfg(test)]
fn tag_to_vec(ctx: &crate::CryptoCtx, symmetric_tag: Handle) -> Result<Vec<u8>, CryptoError> {
    let mut bytes = vec![0u8; ctx.symmetric_tag_len(symmetric_tag)?];
    ctx.symmetric_tag_pull(symmetric_tag, &mut bytes)?;
    Ok(bytes)
}

#[test]
fn test_session() {
    use crate::CryptoCtx;

    let ctx = CryptoCtx::new();

    let msg = b"test";
    let mut msg2 = vec![0u8; msg.len()];
    let mut squeezed = [0u8; 32];
    let mut squeezed_2 = [0u8; 32];
    let key_handle = ctx.symmetric_key_generate("XOODYAK-128", None).unwrap();

    let symmetric_state = ctx
        .symmetric_state_open("XOODYAK-128", Some(key_handle), None)
        .unwrap();

    ctx.symmetric_state_absorb(symmetric_state, b"data")
        .unwrap();
    ctx.symmetric_state_squeeze(symmetric_state, &mut squeezed)
        .unwrap();

    let mut ciphertext_with_tag =
        vec![0u8; msg.len() + ctx.symmetric_state_max_tag_len(symmetric_state).unwrap()];
    ctx.symmetric_state_encrypt(symmetric_state, &mut ciphertext_with_tag, msg)
        .unwrap();

    ctx.symmetric_state_absorb(symmetric_state, b"more_data")
        .unwrap();

    ctx.symmetric_state_ratchet(symmetric_state).unwrap();

    ctx.symmetric_state_squeeze(symmetric_state, &mut squeezed)
        .unwrap();
    ctx.symmetric_state_close(symmetric_state).unwrap();

    //

    let symmetric_state = ctx
        .symmetric_state_open("XOODYAK-128", Some(key_handle), None)
        .unwrap();
    ctx.symmetric_state_absorb(symmetric_state, b"data")
        .unwrap();
    ctx.symmetric_state_squeeze(symmetric_state, &mut squeezed_2)
        .unwrap();

    ctx.symmetric_state_decrypt(symmetric_state, &mut msg2, &ciphertext_with_tag)
        .unwrap();

    ctx.symmetric_state_absorb(symmetric_state, b"more_data")
        .unwrap();

    ctx.symmetric_state_ratchet(symmetric_state).unwrap();

    ctx.symmetric_state_squeeze(symmetric_state, &mut squeezed_2)
        .unwrap();
    ctx.symmetric_state_close(symmetric_state).unwrap();
    assert_eq!(squeezed, squeezed_2);
}
