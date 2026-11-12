//! The shadow bytes of linear memories, for watchpoints.
//!
//! When `Tunables::memory_watchpoints` is enabled, compiled code checks a
//! memory's shadow before every store: a nonzero shadow byte calls a
//! watchpoint builtin before the corresponding memory byte is written. The
//! bits of a shadow byte say why it is watched.

use crate::prelude::*;
use crate::runtime::vm::memory::DefaultMemoryCreator;
use crate::runtime::vm::vmcontext::VMMemoryDefinition;
use crate::{LinearMemory, MemoryCreator, MemoryRequest};
use core::ptr::NonNull;
use wasmtime_environ::MemoryTunables;

/// A shadow bit set for each byte a debugger watches.
#[cfg(feature = "debug")]
pub const WATCH_DEBUG: u8 = 1 << 0;

/// The shadow of a linear memory: one byte per byte of the memory.
///
/// The shadow is itself a linear memory, of the same type and allocated with
/// the same tunables as the memory it shadows, and grows along with it. Its
/// allocation may be larger than the memory (see `reserve_to`), but its size, as
/// seen by compiled code and by `bytes`, is always the memory's.
/// Compiled code accesses the shadow just like any other linear memory, with
/// its own bounds check, at the same Wasm address as the memory access.
///
/// This is owned by the memory's `LocalMemory`, and so is freed along with the
/// memory itself, when the store deallocates the instance that defines the
/// memory. Instances that import the memory only hold pointers to it, just as
/// they do to the memory's `VMMemoryDefinition`, and are deallocated with the
/// same store.
pub struct ShadowMemory {
    alloc: Box<dyn LinearMemory>,
    // Compiled code reaches the shadow through this definition, which has a
    // stable address even when the shadow's base moves or its size changes.
    // Every instance that uses the memory, whether it defines or imports it,
    // points to this one definition.
    definition: Box<VMMemoryDefinition>,
    // Whether compiled code reloads the shadow's base, which otherwise must
    // never move.
    may_move: bool,
    // The size of the memory, and so of the shadow. At most
    // `alloc.byte_size()`.
    len: usize,
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
        let request = MemoryRequest::new(ty, memory_tunables, len, maximum);
        let alloc = DefaultMemoryCreator.new_memory(&request)?;
        let definition = try_new::<Box<_>>(super::vmmemory(&*alloc))?;
        Ok(ShadowMemory {
            len: alloc.byte_size(),
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
        unsafe { core::slice::from_raw_parts(self.alloc.as_ptr(), self.byte_size()) }
    }

    /// The shadow bytes, one per byte of the memory.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: as for `bytes`.
        unsafe { core::slice::from_raw_parts_mut(self.alloc.as_ptr(), self.byte_size()) }
    }

    pub(super) fn byte_size(&self) -> usize {
        self.len
    }

    /// Grows the shadow's allocation to at least `len` bytes, leaving the
    /// actual wasm-reported size of memory unchanged.
    ///
    /// This method is used to perform shadow memory growth before linear memory
    /// growth, so in the event of the latter failing the shadow memory is at
    /// least ready to grow but the extra accessible bytse are harmless.
    pub(super) fn reserve_to(&mut self, len: usize) -> Result<()> {
        if !self.may_move && len > self.alloc.byte_capacity() {
            bail!("Memory maximum size exceeded");
        }
        if len > self.alloc.byte_size() {
            self.alloc.grow_to(len)?;
            *self.definition = self.vmmemory();
        }
        Ok(())
    }

    /// Follows the memory's growth to `len` bytes, after a `reserve_to(len)`.
    pub(super) fn set_byte_size(&mut self, len: usize) {
        assert!(self.len <= len && len <= self.alloc.byte_size());
        self.len = len;
        *self.definition = self.vmmemory();
    }

    /// The shadow's definition: its allocation's base, and the memory's size.
    fn vmmemory(&self) -> VMMemoryDefinition {
        let mut definition = super::vmmemory(&*self.alloc);
        definition.current_length = self.len.into();
        definition
    }
}
