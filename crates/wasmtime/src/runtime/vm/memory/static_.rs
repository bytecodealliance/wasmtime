//! Support for implementing the [`LinearMemory`] trait in terms of a
//! fixed allocation that cannot move.

use crate::prelude::*;
use crate::runtime::vm::{MemoryImageSlot, MmapOffset, SendSyncPtr};
use crate::LinearMemory;

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

    /// The slot of the pooling allocator in which this memory lives.
    ///
    /// The slot has already been instantiated for this memory, including
    /// mapping its CoW image, if any. It manages which bytes of the slot are
    /// accessible as this memory grows, and is handed back to the pooling
    /// allocator with [`StaticMemory::into_image_slot`] when this memory is
    /// deallocated.
    image_slot: MemoryImageSlot,
}

impl StaticMemory {
    /// Create a new static memory at the given memory map location.
    /// The `image_slot` must be already instantiated *for this memory*.
    /// So the `slot.base` is the same location as `base`, `slot.static_size` >= `base_capacity`,
    /// `initial_size` is accessible in the `slot`, and when relying on guards no more than
    /// `initial_size` rounded to full host pages may be accessible.
    pub fn new(
        base: MmapOffset,
        base_capacity: usize,
        initial_size: usize,
        maximum_size: Option<usize>,
        image_slot: MemoryImageSlot,
    ) -> Result<Self> {
        if base_capacity < initial_size {
            bail!(
                "initial memory size of {initial_size} exceeds the pooling allocator's \
                 configured maximum memory size of {base_capacity} bytes",
            );
        }

        // Only use the part of the slice that is necessary.
        let base_capacity = match maximum_size {
            Some(max) if max < base_capacity => max,
            _ => base_capacity,
        };

        Ok(Self {
            base: base.as_non_null().into(),
            capacity: base_capacity,
            size: initial_size,
            image_slot,
        })
    }

    /// Returns the slot state of this memory back to the pooling allocator.
    pub fn into_image_slot(self) -> MemoryImageSlot {
        self.image_slot
    }
}

// SAFETY:
// Note that these points, in particular alignment, rely on the behavior of the pooling allocator.
//
// - **Valid region.** `base` points into the pooling allocator's mapping at the slot owned
// exclusively by this memory (the pool's index allocator hands each slot to one memory at a time).
// `image_slot` manages that same region and keeps the first `size` bytes, rounded up to the host
// page size, accessible. It also holds a reference to the pool's mapping, which keeps
// `base` valid for this memory's lifetime.
//
// - **Size.** Is exactly `initial_size` at creation and is set to exactly `new_byte_size` on growth.
//
// - **Base pointer stability.** `base` never changes. Growth beyond `capacity` is rejected by the
// assertion in `grow_to`, and `LocalMemory` never requests it since `Memory::new_static` forces
// `memory_may_move = false`.
//
// - **Base pointer alignment.** The slot's base is the pool's mapping plus the pre-slab guard plus
// a multiple of the slot size, all of which are multiples of the host page size, so `base` is
// host-page-aligned.
//
// - **Inaccessible region.** This relies on the guard mechanism outlined in the `MemoryPool` documentation.
unsafe impl LinearMemory for StaticMemory {
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

        // Let the slot make the new bytes accessible, then update our
        // accounting of the available size.
        self.image_slot.set_heap_limit(new_byte_size)?;
        self.size = new_byte_size;
        Ok(())
    }

    fn as_ptr(&self) -> *mut u8 {
        self.base.as_ptr()
    }
}
