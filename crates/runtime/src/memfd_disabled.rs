//! Shims for MemFdSlot when the memfd allocator is not
//! included. Enables unconditional use of the type and its methods
//! throughout higher-level code.

use crate::InstantiationError;
use anyhow::Result;
use std::sync::Arc;
use wasmtime_environ::{DefinedMemoryIndex, Module};

/// A shim for the memfd image container when memfd support is not
/// included.
pub enum ModuleMemFds {}

/// A shim for an individual memory image.
#[allow(dead_code)]
pub enum MemoryMemFd {}

impl ModuleMemFds {
    /// Construct a new set of memfd images. This variant is used
    /// when memfd support is not included; it always returns no
    /// images.
    pub fn new(_: &Module, _: &[u8]) -> Result<Option<ModuleMemFds>> {
        Ok(None)
    }

    /// Get the memfd image for a particular memory.
    pub fn get_memory_image(&self, _: DefinedMemoryIndex) -> Option<&Arc<MemoryMemFd>> {
        // Should be unreachable because the `Self` type is
        // uninhabitable.
        match *self {}
    }
}

/// A placeholder for MemFdSlot when we have not included the pooling
/// allocator.
///
/// To allow MemFdSlot to be unconditionally passed around in various
/// places (e.g. a `Memory`), we define a zero-sized type when memfd is
/// not included in the build.
#[derive(Debug)]
pub enum MemFdSlot {}

#[allow(dead_code)]
impl MemFdSlot {
    pub(crate) fn create(_: *mut libc::c_void, _: usize, _: usize) -> Self {
        panic!("create() on invalid MemFdSlot");
    }

    pub(crate) fn instantiate(
        &mut self,
        _: usize,
        _: Option<&Arc<MemoryMemFd>>,
    ) -> Result<Self, InstantiationError> {
        match *self {}
    }

    pub(crate) fn no_clear_on_drop(&mut self) {
        match *self {}
    }

    pub(crate) fn clear_and_remain_ready(&mut self) -> Result<()> {
        match *self {}
    }

    pub(crate) fn has_image(&self) -> bool {
        match *self {}
    }

    pub(crate) fn is_dirty(&self) -> bool {
        match *self {}
    }

    pub(crate) fn set_heap_limit(&mut self, _: usize) -> Result<()> {
        match *self {}
    }
}
