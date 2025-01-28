//! Noop implementations of MPK primitives for environments that do not support
//! the feature.

#![allow(missing_docs)]

#[cfg(feature = "pooling-allocator")]
use crate::prelude::*;

#[cfg(feature = "pooling-allocator")]
pub fn is_supported() -> bool {
    false
}

#[cfg(feature = "pooling-allocator")]
pub fn keys(_: usize) -> &'static [ProtectionKey] {
    &[]
}

pub fn allow(_: ProtectionMask) {}

pub fn current_mask() -> ProtectionMask {
    ProtectionMask
}

#[derive(Clone, Copy, Debug)]
pub enum ProtectionKey {}
impl ProtectionKey {
    #[cfg(feature = "pooling-allocator")]
    pub fn protect(&self, _: &mut [u8]) -> Result<()> {
        match *self {}
    }
    #[cfg(feature = "pooling-allocator")]
    pub fn as_stripe(&self) -> usize {
        match *self {}
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProtectionMask;
impl ProtectionMask {
    pub fn all() -> Self {
        Self
    }
    #[cfg(feature = "pooling-allocator")]
    pub fn zero() -> Self {
        Self
    }
    #[cfg(feature = "pooling-allocator")]
    pub fn or(self, _: ProtectionKey) -> Self {
        Self
    }
}
