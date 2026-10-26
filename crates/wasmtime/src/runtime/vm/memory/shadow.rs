//! The shadow bytes of linear memories, for watchpoints.
//!
//! When `Tunables::memory_watchpoints` is enabled, compiled code checks a
//! memory's shadow before every store: a nonzero shadow byte calls a
//! watchpoint builtin before the corresponding memory byte is written. The
//! bits of a shadow byte say why it is watched.

use crate::prelude::*;
use crate::runtime::vm::memory::{DefaultMemoryCreator, RuntimeLinearMemory, RuntimeMemoryCreator};
use crate::runtime::vm::vmcontext::VMMemoryDefinition;
use core::ptr::NonNull;
use wasmtime_environ::MemoryTunables;

/// A shadow bit set for each byte a debugger watches.
#[cfg(feature = "debug")]
pub const WATCH_DEBUG: u8 = 1 << 0;

/// The shadow of a linear memory: one byte per byte of the memory.
///
/// The shadow is itself a linear memory, of the same type and allocated with
/// the same tunables as the memory it shadows, and grows along with it.
/// Compiled code accesses the shadow just like any other linear memory, with
/// its own bounds check, at the same Wasm address as the memory access.
///
/// This is owned by the memory's `LocalMemory`, and so is freed along with the
/// memory itself, when the store deallocates the instance that defines the
/// memory. Instances that import the memory only hold pointers to it, just as
/// they do to the memory's `VMMemoryDefinition`, and are deallocated with the
/// same store.
pub struct ShadowMemory {
    alloc: Box<dyn RuntimeLinearMemory>,
    // Compiled code reaches the shadow through this definition, which has a
    // stable address even when the shadow's base moves or its size changes.
    // Every instance that uses the memory, whether it defines or imports it,
    // points to this one definition.
    definition: Box<VMMemoryDefinition>,
    // Whether compiled code reloads the shadow's base, which otherwise must
    // never move.
    may_move: bool,
}

impl ShadowMemory {
    /// Creates an unwatched shadow for a memory of type `ty` and `len` bytes.
    pub fn new(
        ty: &wasmtime_environ::Memory,
        memory_tunables: &MemoryTunables<'_>,
        len: usize,
    ) -> Result<Self> {
        let maximum = ty
            .maximum_byte_size()
            .ok()
            .and_then(|n| usize::try_from(n).ok());
        let alloc = DefaultMemoryCreator.new_memory(ty, memory_tunables, len, maximum)?;
        let definition = try_new::<Box<_>>(alloc.vmmemory())?;
        Ok(ShadowMemory {
            alloc,
            definition,
            may_move: ty.memory_may_move(memory_tunables),
        })
    }

    /// The definition through which compiled code finds the shadow bytes.
    pub fn vmmemory_ptr(&self) -> NonNull<VMMemoryDefinition> {
        NonNull::from(&*self.definition)
    }

    /// The shadow bytes, one per byte of the memory.
    pub fn bytes(&self) -> &[u8] {
        // SAFETY: the allocation's first `byte_size` bytes are accessible,
        // and only borrowed through this shadow.
        unsafe { core::slice::from_raw_parts(self.alloc.base().as_mut_ptr(), self.byte_size()) }
    }

    /// The shadow bytes, one per byte of the memory.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: as for `bytes`.
        unsafe { core::slice::from_raw_parts_mut(self.alloc.base().as_mut_ptr(), self.byte_size()) }
    }

    pub(super) fn byte_size(&self) -> usize {
        self.alloc.byte_size()
    }

    /// Whether the shadow can follow the memory's growth to `len` bytes.
    pub(super) fn can_grow_to(&self, len: usize) -> bool {
        self.may_move || len <= self.alloc.byte_capacity()
    }

    /// Follows the memory's growth to `len` bytes.
    pub(super) fn grow_to(&mut self, len: usize) -> Result<()> {
        self.alloc.grow_to(len)?;
        *self.definition = self.alloc.vmmemory();
        Ok(())
    }
}
