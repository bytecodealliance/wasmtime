//! Small shims for CoW support when virtual memory is disabled, meaning that
//! none of the types in this module are supported.

#![warn(dead_code, unused_imports)]

use crate::Engine;
use crate::prelude::*;
use crate::vm::ModuleMemoryImageSource;
use alloc::sync::Arc;
use wasmtime_environ::{DefinedMemoryIndex, Module};

pub enum ModuleMemoryImages {}

impl ModuleMemoryImages {
    pub fn get_memory_image(
        &self,
        _defined_index: DefinedMemoryIndex,
    ) -> Option<&Arc<MemoryImage>> {
        None
    }
}

#[derive(Debug, PartialEq)]
pub enum MemoryImage {}

impl MemoryImage {
    pub(crate) fn linear_memory_offset(&self) -> usize {
        match *self {}
    }

    pub(crate) fn len(&self) -> usize {
        match *self {}
    }

    pub(crate) fn data(&self) -> &[u8] {
        match *self {}
    }

    pub(crate) unsafe fn map_at_ptr(&self, _base: *mut u8) -> Result<()> {
        match *self {}
    }
}

impl ModuleMemoryImages {
    pub fn new(
        _engine: &Engine,
        _module: &Module,
        _source: &Arc<impl ModuleMemoryImageSource>,
    ) -> Result<Option<ModuleMemoryImages>> {
        Ok(None)
    }
}
