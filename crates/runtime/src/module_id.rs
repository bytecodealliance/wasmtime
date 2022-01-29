//! Unique IDs for modules in the runtime.

use std::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

/// A unique identifier (within an engine or similar) for a compiled
/// module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompiledModuleId(NonZeroU64);

/// An allocator for compiled module IDs.
pub struct CompiledModuleIdAllocator {
    next: AtomicU64,
}

impl CompiledModuleIdAllocator {
    /// Create a compiled-module ID allocator.
    pub fn new() -> Self {
        Self {
            next: AtomicU64::new(1),
        }
    }

    /// Allocate a new ID.
    pub fn alloc(&self) -> CompiledModuleId {
        // Note: why is `Relaxed` OK here?
        //
        // The only requirement we have is that IDs are unique. We
        // don't care how one module's ID compares to another, i.e.,
        // what order they come in. `Relaxed` means that this
        // `fetch_add` operation does not have any particular
        // synchronization (ordering) with respect to any other memory
        // access in the program. However, `fetch_add` is always
        // atomic with respect to other accesses to this variable
        // (`self.next`). So we will always hand out separate, unique
        // IDs correctly, just in some possibly arbitrary order (which
        // is fine).
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        CompiledModuleId(NonZeroU64::new(id).unwrap())
    }
}
