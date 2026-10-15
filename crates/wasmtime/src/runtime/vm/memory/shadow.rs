//! The shadow bytes of linear memories, for watchpoints.
//!
//! When `Tunables::memory_watchpoints` is enabled, compiled code checks a
//! memory's shadow before every store: a nonzero shadow byte calls a
//! watchpoint builtin before the corresponding memory byte is written. The
//! bits of a shadow byte say why it is watched.

use crate::prelude::*;
use crate::runtime::vm::VmPtr;
use crate::runtime::vm::vmcontext::VMDefinedShadowMemory;
use core::ptr::NonNull;
use wasmtime_core::alloc::new_zeroed_boxed_slice;

/// A shadow bit set for each byte a debugger watches.
#[cfg(feature = "debug")]
pub const WATCH_DEBUG: u8 = 1 << 0;

/// The shadow of a linear memory: one byte per byte of the memory.
///
/// This is owned by the memory's `LocalMemory`, and so is freed along with the
/// memory itself, when the store deallocates the instance that defines the
/// memory. Instances that import the memory only hold pointers to it, just as
/// they do to the memory's `VMMemoryDefinition`, and are deallocated with the
/// same store.
pub struct ShadowMemory {
    // Compiled code reaches the bytes through this definition, which has a
    // stable address even when the bytes are reallocated. Every instance that
    // uses the memory, whether it defines or imports it, points to this one
    // definition, so updating its base here updates it for all of them.
    definition: Box<VMDefinedShadowMemory>,
    bytes: Box<[u8]>,
    len: usize,
}

// SAFETY: the definition only points to `bytes`, which the shadow owns.
unsafe impl Send for ShadowMemory {}
unsafe impl Sync for ShadowMemory {}

impl ShadowMemory {
    /// Creates an unwatched shadow for a memory of `len` bytes.
    pub fn new(len: usize) -> Result<Self> {
        let bytes = zeroed(len)?;
        let definition = try_new::<Box<_>>(VMDefinedShadowMemory {
            base: VmPtr::from(base(&bytes)),
        })?;
        Ok(ShadowMemory {
            definition,
            bytes,
            len,
        })
    }

    /// The definition through which compiled code finds the shadow bytes.
    pub fn vmshadow(&self) -> NonNull<VMDefinedShadowMemory> {
        NonNull::from(&*self.definition)
    }

    /// The shadow bytes, one per byte of the memory.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// The shadow bytes, one per byte of the memory.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes[..self.len]
    }

    /// Follows the memory's size, which is now `len` bytes.
    pub fn resize(&mut self, len: usize) -> Result<()> {
        if len > self.bytes.len() {
            let mut bytes = zeroed(len.max(self.bytes.len().saturating_mul(2)))?;
            bytes[..self.len].copy_from_slice(&self.bytes[..self.len]);
            self.definition.base = VmPtr::from(base(&bytes));
            self.bytes = bytes;
        }
        if len > self.len {
            self.bytes[self.len..len].fill(0);
        }
        self.len = len;
        Ok(())
    }
}

fn base(bytes: &[u8]) -> NonNull<u8> {
    NonNull::new(bytes.as_ptr().cast_mut()).unwrap_or(NonNull::dangling())
}

/// Allocates zeroed bytes, which the allocator can provide lazily.
fn zeroed(len: usize) -> Result<Box<[u8]>, OutOfMemory> {
    let bytes = new_zeroed_boxed_slice::<u8>(len)?;
    // SAFETY: all-zero bytes are valid `u8`s.
    Ok(unsafe { bytes.assume_init() })
}
