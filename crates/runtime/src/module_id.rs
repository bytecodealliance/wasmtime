//! Unique IDs for modules in the runtime.

use std::sync::atomic::{AtomicU64, Ordering};

/// A unique identifier (within an engine or similar) for a compiled
/// module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompiledModuleId(u64);

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
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        CompiledModuleId(id)
    }
}
