//! Noop implementations of MPK primitives for environments that do not support
//! the feature.

#![allow(missing_docs)]

use anyhow::Result;

pub fn is_supported() -> bool {
    false
}
pub fn keys() -> &'static [ProtectionKey] {
    &[]
}
pub fn allow(_: ProtectionMask) {}

#[derive(Clone, Copy, Debug)]
pub struct ProtectionKey;
impl ProtectionKey {
    pub fn protect(&self, _: &mut [u8]) -> Result<()> {
        Ok(())
    }
    pub fn as_stripe(&self) -> usize {
        0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProtectionMask;
impl ProtectionMask {
    pub fn all() -> Self {
        Self
    }
    pub fn zero() -> Self {
        Self
    }
    pub fn or(self, _: ProtectionKey) -> Self {
        Self
    }
}
