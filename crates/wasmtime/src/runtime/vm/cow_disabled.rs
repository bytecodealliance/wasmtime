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

impl ModuleMemoryImages {
    pub fn new(
        _engine: &Engine,
        _module: &Module,
        _source: &Arc<impl ModuleMemoryImageSource>,
    ) -> Result<Option<ModuleMemoryImages>> {
        Ok(None)
    }
}

#[derive(Debug)]
pub enum MemoryImageSlot {}

impl MemoryImageSlot {
    pub(crate) fn set_heap_limit(&mut self, _size_bytes: usize) -> Result<()> {
        match *self {}
    }

    pub(crate) fn has_image(&self) -> bool {
        match *self {}
    }
}
