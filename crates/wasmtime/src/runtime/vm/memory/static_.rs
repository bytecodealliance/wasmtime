//! Support for implementing the [`RuntimeLinearMemory`] trait in terms of a
//! fixed allocation that cannot move.

use crate::prelude::*;
use crate::runtime::vm::memory::RuntimeLinearMemory;
use crate::runtime::vm::{MemoryBase, SendSyncPtr};
use core::ptr::NonNull;

/// A "static" memory where the lifetime of the backing memory is managed
/// elsewhere. Currently used with the pooling allocator.
pub struct StaticMemory {
    /// The base pointer of this static memory, wrapped up in a send/sync
    /// wrapper.
    base: SendSyncPtr<u8>,

    /// The byte capacity of the `base` pointer.
    capacity: usize,

    /// The current size, in bytes, of this memory.
    size: usize,
}

impl StaticMemory {
    pub fn new(
        base_ptr: *mut u8,
        base_capacity: usize,
        initial_size: usize,
        maximum_size: Option<usize>,
    ) -> Result<Self> {
        if base_capacity < initial_size {
            bail!(
                "initial memory size of {} exceeds the pooling allocator's \
                 configured maximum memory size of {} bytes",
                initial_size,
                base_capacity,
            );
        }

        // Only use the part of the slice that is necessary.
        let base_capacity = match maximum_size {
            Some(max) if max < base_capacity => max,
            _ => base_capacity,
        };

        Ok(Self {
            base: SendSyncPtr::new(NonNull::new(base_ptr).unwrap()),
            capacity: base_capacity,
            size: initial_size,
        })
    }
}

impl RuntimeLinearMemory for StaticMemory {
    fn byte_size(&self) -> usize {
        self.size
    }

    fn byte_capacity(&self) -> usize {
        self.capacity
    }

    fn grow_to(&mut self, new_byte_size: usize) -> Result<()> {
        // Never exceed the static memory size; this check should have been made
        // prior to arriving here.
        assert!(new_byte_size <= self.capacity);

        // Update our accounting of the available size.
        self.size = new_byte_size;
        Ok(())
    }

    fn set_byte_size(&mut self, len: usize) {
        self.size = len;
    }

    fn base(&self) -> MemoryBase<'_> {
        // XXX: Returning a raw pointer isn't quite right. A `StaticMemory` is
        // usually created via a pre-existing `Mmap` instance, but we lose that
        // info since we store a raw pointer above. However, retaining that info
        // is tricky because it would introduce a lifetime param into
        // `StaticMemory`.
        //
        // One solution would be to store `Arc<Mmap>`.
        MemoryBase::new_raw(self.base.as_ptr())
    }
}
