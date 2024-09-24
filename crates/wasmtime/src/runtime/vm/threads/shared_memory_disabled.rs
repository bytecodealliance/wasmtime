#![allow(missing_docs)]

use crate::prelude::*;
use crate::runtime::vm::{RuntimeLinearMemory, VMMemoryDefinition, VMStore, WaitResult};
use core::ops::Range;
use core::time::Duration;
use wasmtime_environ::{MemoryPlan, Trap};

#[derive(Clone)]
pub enum SharedMemory {}

impl SharedMemory {
    pub fn wrap(
        _plan: &MemoryPlan,
        _memory: Box<dyn RuntimeLinearMemory>,
        _ty: wasmtime_environ::Memory,
    ) -> Result<Self> {
        bail!("support for shared memories was disabled at compile time");
    }

    pub fn ty(&self) -> wasmtime_environ::Memory {
        match *self {}
    }

    pub fn as_memory(self) -> crate::runtime::vm::Memory {
        match self {}
    }

    pub fn vmmemory_ptr(&self) -> *const VMMemoryDefinition {
        match *self {}
    }

    pub fn grow(
        &self,
        _delta_pages: u64,
        _store: Option<&mut dyn VMStore>,
    ) -> Result<Option<(usize, usize)>> {
        match *self {}
    }

    pub fn atomic_notify(&self, _addr_index: u64, _count: u32) -> Result<u32, Trap> {
        match *self {}
    }

    pub fn atomic_wait32(
        &self,
        _addr_index: u64,
        _expected: u32,
        _timeout: Option<Duration>,
    ) -> Result<WaitResult, Trap> {
        match *self {}
    }

    pub fn atomic_wait64(
        &self,
        _addr_index: u64,
        _expected: u64,
        _timeout: Option<Duration>,
    ) -> Result<WaitResult, Trap> {
        match *self {}
    }
}

impl RuntimeLinearMemory for SharedMemory {
    fn page_size_log2(&self) -> u8 {
        match *self {}
    }

    fn byte_size(&self) -> usize {
        match *self {}
    }

    fn maximum_byte_size(&self) -> Option<usize> {
        match *self {}
    }

    fn grow(
        &mut self,
        _delta_pages: u64,
        _store: Option<&mut dyn VMStore>,
    ) -> Result<Option<(usize, usize)>> {
        match *self {}
    }

    fn grow_to(&mut self, _size: usize) -> Result<()> {
        match *self {}
    }

    fn vmmemory(&mut self) -> VMMemoryDefinition {
        match *self {}
    }

    fn needs_init(&self) -> bool {
        match *self {}
    }

    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        match *self {}
    }

    fn wasm_accessible(&self) -> Range<usize> {
        match *self {}
    }
}
