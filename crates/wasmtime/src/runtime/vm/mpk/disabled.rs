//! Noop implementations of MPK primitives for environments that do not support
//! the feature.

#[cfg(any(feature = "pooling-allocator", has_virtual_memory))]
use crate::prelude::*;

#[cfg(feature = "pooling-allocator")]
pub fn is_supported() -> bool {
    false
}

#[cfg(feature = "pooling-allocator")]
pub fn keys(_: usize) -> &'static [ProtectionKey] {
    &[]
}

#[cfg(any(feature = "async", feature = "pooling-allocator"))]
pub fn allow(_: ProtectionMask) {}

#[cfg(feature = "async")]
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
    // Note: gated on `has_virtual_memory` rather than `pooling-allocator`
    // because this is called from `cow.rs`, which is not pooling-specific.
    #[cfg(has_virtual_memory)]
    pub unsafe fn reprotect(&self, _: usize, _: usize, _: bool) -> Result<()> {
        match *self {}
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg(any(feature = "async", feature = "pooling-allocator"))]
pub struct ProtectionMask;

#[cfg(any(feature = "async", feature = "pooling-allocator"))]
impl ProtectionMask {
    #[cfg(any(feature = "async", feature = "pooling-allocator"))]
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
