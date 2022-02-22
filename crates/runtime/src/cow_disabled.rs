//! Shims for MemoryImageSlot when the copy-on-write memory initialization is
//! not included. Enables unconditional use of the type and its methods
//! throughout higher-level code.

use crate::{InstantiationError, MmapVec};
use anyhow::Result;
use std::sync::Arc;
use wasmtime_environ::{DefinedMemoryIndex, Module};

/// A shim for the memory image container when support is not included.
pub enum ModuleMemoryImages {}

/// A shim for an individual memory image.
#[allow(dead_code)]
pub enum MemoryImage {}

impl ModuleMemoryImages {
    /// Construct a new set of memory images. This variant is used
    /// when cow support is not included; it always returns no
    /// images.
    pub fn new(_: &Module, _: &[u8], _: Option<&MmapVec>) -> Result<Option<ModuleMemoryImages>> {
        Ok(None)
    }

    /// Get the memory image for a particular memory.
    pub fn get_memory_image(&self, _: DefinedMemoryIndex) -> Option<&Arc<MemoryImage>> {
        match *self {}
    }
}

/// A placeholder for MemoryImageSlot when we have not included the pooling
/// allocator.
///
/// To allow MemoryImageSlot to be unconditionally passed around in various
/// places (e.g. a `Memory`), we define a zero-sized type when memory is
/// not included in the build.
#[derive(Debug)]
pub enum MemoryImageSlot {}

#[allow(dead_code)]
impl MemoryImageSlot {
    pub(crate) fn create(_: *mut libc::c_void, _: usize, _: usize) -> Self {
        panic!("create() on invalid MemoryImageSlot");
    }

    pub(crate) fn instantiate(
        &mut self,
        _: usize,
        _: Option<&Arc<MemoryImage>>,
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
