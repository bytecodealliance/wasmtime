//! Noop implementations of MPK primitives for environments that do not support
//! the feature.

#![allow(missing_docs)]

use crate::prelude::*;

pub fn is_supported() -> bool {
    false
}
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
    pub fn protect(&self, _: &mut [u8]) -> Result<()> {
        match *self {}
    }
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
    pub fn zero() -> Self {
        Self
    }
    pub fn or(self, _: ProtectionKey) -> Self {
        Self
    }
}
