use super::*;
use crate::CryptoCtx;

use subtle::ConstantTimeEq;
use zeroize::Zeroize;

#[derive(Debug, Clone, Eq)]
pub struct SymmetricTag {
    alg: SymmetricAlgorithm,
    raw: Vec<u8>,
}

impl PartialEq for SymmetricTag {
    fn eq(&self, other: &Self) -> bool {
        self.alg == other.alg && self.raw.ct_eq(&other.raw).unwrap_u8() == 1
    }
}

impl Drop for SymmetricTag {
    fn drop(&mut self) {
        self.raw.zeroize();
    }
}

impl SymmetricTag {
    pub fn new(alg: SymmetricAlgorithm, raw: Vec<u8>) -> Self {
        SymmetricTag { alg, raw }
    }

    pub fn verify(&self, expected_raw: &[u8]) -> Result<(), CryptoError> {
        ensure!(
            self.raw.ct_eq(expected_raw).unwrap_u8() == 1,
            CryptoError::InvalidTag
        );
        Ok(())
    }
}

impl AsRef<[u8]> for SymmetricTag {
    fn as_ref(&self) -> &[u8] {
        &self.raw
    }
}

impl CryptoCtx {
    pub fn symmetric_tag_len(&self, symmetric_tag_handle: Handle) -> Result<usize, CryptoError> {
        let symmetric_tag = self.handles.symmetric_tag.get(symmetric_tag_handle)?;
        Ok(symmetric_tag.as_ref().len())
    }

    pub fn symmetric_tag_pull(
        &self,
        symmetric_tag_handle: Handle,
        buf: &mut [u8],
    ) -> Result<usize, CryptoError> {
        let symmetric_tag = self.handles.symmetric_tag.get(symmetric_tag_handle)?;
        let raw = symmetric_tag.as_ref();
        let raw_len = raw.len();
        let buf_len = buf.len();
        ensure!(raw_len <= buf_len, CryptoError::Overflow);
        buf[..raw_len].copy_from_slice(raw);
        self.handles.symmetric_tag.close(symmetric_tag_handle)?;
        Ok(raw_len)
    }

    pub fn symmetric_tag_verify(
        &self,
        symmetric_tag_handle: Handle,
        expected_raw: &[u8],
    ) -> Result<(), CryptoError> {
        let symmetric_tag = self.handles.symmetric_tag.get(symmetric_tag_handle)?;
        symmetric_tag.verify(expected_raw)
    }

    pub fn symmetric_tag_close(&self, symmetric_tag_handle: Handle) -> Result<(), CryptoError> {
        self.handles.symmetric_tag.close(symmetric_tag_handle)
    }
}
