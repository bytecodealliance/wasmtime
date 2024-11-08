//! Support for implementing the [`RuntimeLinearMemory`] trait in terms of a
//! platform mmap primitive.

use crate::prelude::*;
use crate::runtime::vm::memory::RuntimeLinearMemory;
use crate::runtime::vm::mmap::Mmap;
use crate::runtime::vm::{round_usize_up_to_host_pages, usize_is_multiple_of_host_page_size};
use wasmtime_environ::Tunables;

/// A linear memory instance.
#[derive(Debug)]
pub struct MmapMemory {
    // The underlying allocation.
    mmap: Mmap,

    // The current length of this Wasm memory, in bytes.
    //
    // This region starts at `pre_guard_size` offset from the base of `mmap`. It
    // is always accessible, which means that if the Wasm page size is smaller
    // than the host page size, there may be some trailing region in the `mmap`
    // that is accessible but should not be accessed. (We rely on explicit
    // bounds checks in the compiled code to protect this region.)
    len: usize,

    // The optional maximum accessible size, in bytes, for this linear memory.
    //
    // Note that this maximum does not factor in guard pages, so this isn't the
    // maximum size of the linear address space reservation for this memory.
    //
    // This is *not* always a multiple of the host page size, and
    // `self.accessible()` may go past `self.maximum` when Wasm is using a small
    // custom page size due to `self.accessible()`'s rounding up to the host
    // page size.
    maximum: Option<usize>,

    // The amount of extra bytes to reserve whenever memory grows. This is
    // specified so that the cost of repeated growth is amortized.
    extra_to_reserve_on_growth: usize,

    // Size in bytes of extra guard pages before the start and after the end to
    // optimize loads and stores with constant offsets.
    pre_guard_size: usize,
    offset_guard_size: usize,
}

impl MmapMemory {
    /// Create a new linear memory instance with specified minimum and maximum
    /// number of wasm pages.
    pub fn new(
        ty: &wasmtime_environ::Memory,
        tunables: &Tunables,
        minimum: usize,
        maximum: Option<usize>,
    ) -> Result<Self> {
        // It's a programmer error for these two configuration values to exceed
        // the host available address space, so panic if such a configuration is
        // found (mostly an issue for hypothetical 32-bit hosts).
        //
        // Also be sure to round up to the host page size for this value.
        let offset_guard_bytes = usize::try_from(tunables.memory_guard_size).unwrap();
        let offset_guard_bytes = round_usize_up_to_host_pages(offset_guard_bytes)?;
        let pre_guard_bytes = if tunables.guard_before_linear_memory {
            offset_guard_bytes
        } else {
            0
        };

        // Calculate how much is going to be allocated for this linear memory in
        // addition to how much extra space we're reserving to grow into.
        //
        // If the minimum size of this linear memory fits within the initial
        // allocation (tunables.memory_reservation) then that's how many bytes
        // are going to be allocated. If the maximum size of linear memory
        // additionally fits within the entire allocation then there's no need
        // to reserve any extra for growth.
        //
        // If the minimum size doesn't fit within this linear memory.
        let mut alloc_bytes = tunables.memory_reservation;
        let mut extra_to_reserve_on_growth = tunables.memory_reservation_for_growth;
        let minimum_u64 = u64::try_from(minimum).unwrap();
        if minimum_u64 <= alloc_bytes {
            if let Ok(max) = ty.maximum_byte_size() {
                if max <= alloc_bytes {
                    extra_to_reserve_on_growth = 0;
                }
            }
        } else {
            alloc_bytes = minimum_u64.saturating_add(extra_to_reserve_on_growth);
        }

        // Convert `alloc_bytes` and `extra_to_reserve_on_growth` to
        // page-aligned `usize` values.
        let alloc_bytes = usize::try_from(alloc_bytes).unwrap();
        let extra_to_reserve_on_growth = usize::try_from(extra_to_reserve_on_growth).unwrap();
        let alloc_bytes = round_usize_up_to_host_pages(alloc_bytes)?;
        let extra_to_reserve_on_growth = round_usize_up_to_host_pages(extra_to_reserve_on_growth)?;

        let request_bytes = pre_guard_bytes
            .checked_add(alloc_bytes)
            .and_then(|i| i.checked_add(offset_guard_bytes))
            .ok_or_else(|| format_err!("cannot allocate {} with guard regions", minimum))?;
        assert!(usize_is_multiple_of_host_page_size(request_bytes));

        let mut mmap = Mmap::accessible_reserved(0, request_bytes)?;

        if minimum > 0 {
            let accessible = round_usize_up_to_host_pages(minimum)?;
            mmap.make_accessible(pre_guard_bytes, accessible)?;
        }

        Ok(Self {
            mmap,
            len: minimum,
            maximum,
            pre_guard_size: pre_guard_bytes,
            offset_guard_size: offset_guard_bytes,
            extra_to_reserve_on_growth,
        })
    }

    /// Get the length of the accessible portion of the underlying `mmap`. This
    /// is the same region as `self.len` but rounded up to a multiple of the
    /// host page size.
    fn accessible(&self) -> usize {
        let accessible =
            round_usize_up_to_host_pages(self.len).expect("accessible region always fits in usize");
        debug_assert!(accessible <= self.mmap.len() - self.offset_guard_size - self.pre_guard_size);
        accessible
    }
}

impl RuntimeLinearMemory for MmapMemory {
    fn byte_size(&self) -> usize {
        self.len
    }

    fn byte_capacity(&self) -> usize {
        self.mmap.len() - self.offset_guard_size - self.pre_guard_size
    }

    fn grow_to(&mut self, new_size: usize) -> Result<()> {
        assert!(usize_is_multiple_of_host_page_size(self.offset_guard_size));
        assert!(usize_is_multiple_of_host_page_size(self.pre_guard_size));
        assert!(usize_is_multiple_of_host_page_size(self.mmap.len()));

        let new_accessible = round_usize_up_to_host_pages(new_size)?;
        if new_accessible > self.mmap.len() - self.offset_guard_size - self.pre_guard_size {
            // If the new size of this heap exceeds the current size of the
            // allocation we have, then this must be a dynamic heap. Use
            // `new_size` to calculate a new size of an allocation, allocate it,
            // and then copy over the memory from before.
            let request_bytes = self
                .pre_guard_size
                .checked_add(new_accessible)
                .and_then(|s| s.checked_add(self.extra_to_reserve_on_growth))
                .and_then(|s| s.checked_add(self.offset_guard_size))
                .ok_or_else(|| format_err!("overflow calculating size of memory allocation"))?;
            assert!(usize_is_multiple_of_host_page_size(request_bytes));

            let mut new_mmap = Mmap::accessible_reserved(0, request_bytes)?;
            new_mmap.make_accessible(self.pre_guard_size, new_accessible)?;

            // This method has an exclusive reference to `self.mmap` and just
            // created `new_mmap` so it should be safe to acquire references
            // into both of them and copy between them.
            unsafe {
                let range = self.pre_guard_size..self.pre_guard_size + self.len;
                let src = self.mmap.slice(range.clone());
                let dst = new_mmap.slice_mut(range);
                dst.copy_from_slice(src);
            }

            self.mmap = new_mmap;
        } else {
            // If the new size of this heap fits within the existing allocation
            // then all we need to do is to make the new pages accessible. This
            // can happen either for "static" heaps which always hit this case,
            // or "dynamic" heaps which have some space reserved after the
            // initial allocation to grow into before the heap is moved in
            // memory.
            assert!(new_size > self.len);
            assert!(self.maximum.map_or(true, |max| new_size <= max));
            assert!(new_size <= self.mmap.len() - self.offset_guard_size - self.pre_guard_size);

            let new_accessible = round_usize_up_to_host_pages(new_size)?;
            assert!(
                new_accessible <= self.mmap.len() - self.offset_guard_size - self.pre_guard_size,
            );

            // If the Wasm memory's page size is smaller than the host's page
            // size, then we might not need to actually change permissions,
            // since we are forced to round our accessible range up to the
            // host's page size.
            if new_accessible > self.accessible() {
                self.mmap.make_accessible(
                    self.pre_guard_size + self.accessible(),
                    new_accessible - self.accessible(),
                )?;
            }
        }

        self.len = new_size;

        Ok(())
    }

    fn set_byte_size(&mut self, len: usize) {
        self.len = len;
    }

    fn base_ptr(&self) -> *mut u8 {
        unsafe { self.mmap.as_mut_ptr().add(self.pre_guard_size) }
    }
}
