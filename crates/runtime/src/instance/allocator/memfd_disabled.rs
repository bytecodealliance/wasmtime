//! Shims for MemFdSlot when the memfd allocator is not
//! included. Enables unconditional use of the type and its methods
//! throughout higher-level code.

use crate::InstantiationError;
use anyhow::Result;
use std::sync::Arc;

/// A placeholder for MemFdSlot when we have not included the pooling
/// allocator.
///
/// To allow MemFdSlot to be unconditionally passed around in various
/// places (e.g. a `Memory`), we define a zero-sized type when memfd is
/// not included in the build.
#[cfg(not(feature = "memfd-allocator"))]
#[derive(Debug)]
pub struct MemFdSlot;

#[cfg(not(feature = "memfd-allocator"))]
#[allow(dead_code)]
impl MemFdSlot {
    pub(crate) fn create(_: *mut libc::c_void, _: usize) -> Result<Self, InstantiationError> {
        panic!("create() on invalid MemFdSlot");
    }

    pub(crate) fn instantiate(
        &mut self,
        _: usize,
        _: Option<&Arc<crate::memfd::MemoryMemFd>>,
    ) -> Result<Self, InstantiationError> {
        panic!("instantiate() on invalid MemFdSlot");
    }

    pub(crate) fn clear_and_remain_ready(&mut self) -> Result<()> {
        Ok(())
    }

    pub(crate) fn has_image(&self) -> bool {
        false
    }

    pub(crate) fn is_dirty(&self) -> bool {
        false
    }

    pub(crate) fn set_heap_limit(&mut self, _: usize) -> Result<()> {
        panic!("set_heap_limit on invalid MemFdSlot");
    }
}
