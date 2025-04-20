use std::io;
use std::mem::ManuallyDrop;
use std::ptr;

use cranelift_module::ModuleResult;

use super::{BranchProtection, JITMemoryProvider};

fn align_up(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (addr + align - 1) & !(align - 1)
}

#[derive(Debug)]
struct Segment {
    ptr: *mut u8,
    len: usize,
    position: usize,
    target_prot: region::Protection,
    finalized: bool,
}

impl Segment {
    fn new(ptr: *mut u8, len: usize, target_prot: region::Protection) -> Self {
        // Segments are created on page boundaries.
        debug_assert_eq!(ptr as usize % region::page::size(), 0);
        debug_assert_eq!(len % region::page::size(), 0);
        let mut segment = Segment {
            ptr,
            len,
            target_prot,
            position: 0,
            finalized: false,
        };
        // Set segment to read-write for initialization. The target permissions
        // will be applied in `finalize`.
        segment.set_rw();
        segment
    }

    fn set_rw(&mut self) {
        unsafe {
            region::protect(self.ptr, self.len, region::Protection::READ_WRITE)
                .expect("unable to change memory protection for jit memory segment");
        }
    }

    fn finalize(&mut self, branch_protection: BranchProtection) {
        if self.finalized {
            return;
        }

        // Executable regions are handled separately to correctly deal with
        // branch protection and cache coherence.
        if self.target_prot == region::Protection::READ_EXECUTE {
            super::set_readable_and_executable(self.ptr, self.len, branch_protection)
                .expect("unable to set memory protection for jit memory segment");
        } else {
            unsafe {
                region::protect(self.ptr, self.len, self.target_prot)
                    .expect("unable to change memory protection for jit memory segment");
            }
        }
        self.finalized = true;
    }

    // Note: We do pointer arithmetic on `ptr` passed to `Segment::new` here.
    // This assumes that `ptr` is valid for `len` bytes, or will result in UB.
    fn allocate(&mut self, size: usize, align: usize) -> *mut u8 {
        assert!(self.has_space_for(size, align));
        self.position = align_up(self.position, align);
        let ptr = unsafe { self.ptr.add(self.position) };
        self.position += size;
        ptr
    }

    fn has_space_for(&self, size: usize, align: usize) -> bool {
        !self.finalized && align_up(self.position, align) + size <= self.len
    }
}

/// `ArenaMemoryProvider` allocates segments from a contiguous memory region
/// that is reserved up-front.
///
/// The arena's memory is initially allocated with PROT_NONE and gradually
/// updated as the JIT requires more space. This approach allows for stable
/// addresses throughout the lifetime of the JIT.
///
/// Depending on the underlying platform, requesting large parts of the address
/// space to be allocated might fail. This implementation currently doesn't do
/// overcommit on Windows.
///
/// Note: Memory will be leaked by default unless
/// [`JITMemoryProvider::free_memory`] is called to ensure function pointers
/// remain valid for the remainder of the program's life.
pub struct ArenaMemoryProvider {
    alloc: ManuallyDrop<Option<region::Allocation>>,
    ptr: *mut u8,
    size: usize,
    position: usize,
    segments: Vec<Segment>,
}

impl ArenaMemoryProvider {
    /// Create a new memory region with the given size.
    pub fn new_with_size(reserve_size: usize) -> Result<Self, region::Error> {
        let size = align_up(reserve_size, region::page::size());
        // Note: The region crate uses `MEM_RESERVE | MEM_COMMIT` on Windows.
        // This means that allocations that exceed the page file plus system
        // memory will fail here.
        // https://github.com/darfink/region-rs/pull/34
        let mut alloc = region::alloc(size, region::Protection::NONE)?;
        let ptr = alloc.as_mut_ptr();

        Ok(Self {
            alloc: ManuallyDrop::new(Some(alloc)),
            segments: Vec::new(),
            ptr,
            size,
            position: 0,
        })
    }

    fn allocate(
        &mut self,
        size: usize,
        align: u64,
        protection: region::Protection,
    ) -> io::Result<*mut u8> {
        let align = usize::try_from(align).expect("alignment too big");
        assert!(
            align <= region::page::size(),
            "alignment over page size is not supported"
        );

        // Note: Add a fast path without a linear scan over segments here?

        // Can we fit this allocation into an existing segment?
        if let Some(segment) = self.segments.iter_mut().find(|seg| {
            seg.target_prot == protection && !seg.finalized && seg.has_space_for(size, align)
        }) {
            return Ok(segment.allocate(size, align));
        }

        // Can we resize the last segment?
        if let Some(segment) = self.segments.iter_mut().last() {
            if segment.target_prot == protection && !segment.finalized {
                let additional_size = align_up(size, region::page::size());

                // If our reserved arena can fit the additional size, extend the
                // last segment.
                if self.position + additional_size <= self.size {
                    segment.len += additional_size;
                    segment.set_rw();
                    self.position += additional_size;
                    return Ok(segment.allocate(size, align));
                }
            }
        }

        // Allocate new segment for given size and alignment.
        self.allocate_segment(size, protection)?;
        let i = self.segments.len() - 1;
        Ok(self.segments[i].allocate(size, align))
    }

    fn allocate_segment(
        &mut self,
        size: usize,
        target_prot: region::Protection,
    ) -> Result<(), io::Error> {
        let size = align_up(size, region::page::size());
        let ptr = unsafe { self.ptr.add(self.position) };
        if self.position + size > self.size {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "pre-allocated jit memory region exhausted",
            ));
        }
        self.position += size;
        self.segments.push(Segment::new(ptr, size, target_prot));
        Ok(())
    }

    pub(crate) fn finalize(&mut self, branch_protection: BranchProtection) {
        for segment in &mut self.segments {
            segment.finalize(branch_protection);
        }

        // Flush any in-flight instructions from the pipeline
        wasmtime_jit_icache_coherence::pipeline_flush_mt().expect("Failed pipeline flush");
    }

    /// Frees the allocated memory region, which would be leaked otherwise.
    /// Likely to invalidate existing function pointers, causing unsafety.
    pub(crate) unsafe fn free_memory(&mut self) {
        if self.ptr == ptr::null_mut() {
            return;
        }
        self.segments.clear();
        // Drop the allocation, freeing memory.
        let _: Option<region::Allocation> = self.alloc.take();
        self.ptr = ptr::null_mut();
    }
}

impl Drop for ArenaMemoryProvider {
    fn drop(&mut self) {
        if self.ptr == ptr::null_mut() {
            return;
        }
        let is_live = self.segments.iter().any(|seg| seg.finalized);
        if !is_live {
            // Only free memory if it's not been finalized yet.
            // Otherwise, leak it since JIT memory may still be in use.
            unsafe { self.free_memory() };
        }
    }
}

impl JITMemoryProvider for ArenaMemoryProvider {
    fn allocate_readexec(&mut self, size: usize, align: u64) -> io::Result<*mut u8> {
        self.allocate(size, align, region::Protection::READ_EXECUTE)
    }

    fn allocate_readwrite(&mut self, size: usize, align: u64) -> io::Result<*mut u8> {
        self.allocate(size, align, region::Protection::READ_WRITE)
    }

    fn allocate_readonly(&mut self, size: usize, align: u64) -> io::Result<*mut u8> {
        self.allocate(size, align, region::Protection::READ)
    }

    unsafe fn free_memory(&mut self) {
        self.free_memory();
    }

    fn finalize(&mut self, branch_protection: BranchProtection) -> ModuleResult<()> {
        self.finalize(branch_protection);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_ok() {
        let mut arena = ArenaMemoryProvider::new_with_size(1 << 20).unwrap();

        for align_log2 in 0..8 {
            let align = 1usize << align_log2;
            for size in 1..128 {
                let ptr = arena.allocate_readwrite(size, align as u64).unwrap();
                // assert!(ptr.is_aligned_to(align));
                assert_eq!(ptr.addr() % align, 0);
            }
        }
    }

    #[test]
    #[cfg(all(target_pointer_width = "64", not(target_os = "windows")))]
    // Windows: See https://github.com/darfink/region-rs/pull/34
    fn large_virtual_allocation() {
        // We should be able to request 1TB of virtual address space on 64-bit
        // platforms. Physical memory should be committed as we go.
        let reserve_size = 1 << 40;
        let mut arena = ArenaMemoryProvider::new_with_size(reserve_size).unwrap();
        let ptr = arena.allocate_readwrite(1, 1).unwrap();
        assert_eq!(ptr.addr(), arena.ptr.addr());
        arena.finalize(BranchProtection::None);
        unsafe { ptr.write_volatile(42) };
        unsafe { arena.free_memory() };
    }

    #[test]
    fn over_capacity() {
        let mut arena = ArenaMemoryProvider::new_with_size(1 << 20).unwrap(); // 1 MB

        let _ = arena.allocate_readwrite(900_000, 1).unwrap();
        let _ = arena.allocate_readwrite(200_000, 1).unwrap_err();
    }
}
