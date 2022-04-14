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
        let mut segment = Segment {
            ptr,
            len,
            target_prot,
            position: 0,
            finalized: false,
        };
        // set segment to read-write for initialization
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
        let mut alloc = region::alloc(reserve_size, region::Protection::NONE)?;
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
        align: usize,
        protection: region::Protection,
    ) -> io::Result<*mut u8> {
        // Note: Add a fast path without a linear scan over segments here?

        // can we fit this allocation into an existing segment
        if let Some(segment) = self.segments.iter_mut().find(|seg| {
            seg.target_prot == protection && !seg.finalized && seg.has_space_for(size, align)
        }) {
            return Ok(segment.allocate(size, align));
        }

        // can we resize the last segment?
        if let Some(segment) = self.segments.iter_mut().last() {
            if segment.target_prot == protection && !segment.finalized {
                let align = align.max(region::page::size());
                let additional_size = align_up(size, align);

                // if our reserved arena can fit the additional size, extend the
                // last segment
                if self.position + additional_size <= self.size {
                    segment.len += additional_size;
                    segment.set_rw();
                    self.position += additional_size;
                    return Ok(segment.allocate(size, align));
                }
            }
        }

        // allocate new segment for given size and alignment
        self.allocate_segment(align_up(size, align), protection)?;
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
    }

    /// Frees the allocated memory region, which would be leaked otherwise.
    /// Likely to invalidate existing function pointers, causing unsafety.
    pub(crate) unsafe fn free_memory(&mut self) {
        if self.ptr == ptr::null_mut() {
            return;
        }
        self.segments.clear();
        // Drop the allocation, freeing memory
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
        self.allocate(size, align as usize, region::Protection::READ_EXECUTE)
    }

    fn allocate_readwrite(&mut self, size: usize, align: u64) -> io::Result<*mut u8> {
        self.allocate(size, align as usize, region::Protection::READ_WRITE)
    }

    fn allocate_readonly(&mut self, size: usize, align: u64) -> io::Result<*mut u8> {
        self.allocate(size, align as usize, region::Protection::READ)
    }

    unsafe fn free_memory(&mut self) {
        self.free_memory();
    }

    fn finalize(&mut self, branch_protection: BranchProtection) -> ModuleResult<()> {
        self.finalize(branch_protection);
        Ok(())
    }
}
