//! memfd mapping logic for use by the pooling allocator.

use crate::memfd::MemoryMemFd;
use crate::InstantiationError;
use anyhow::Result;
use libc::c_void;
use rustix::fd::AsRawFd;
use std::sync::Arc;

/// A single slot handled by the memfd instance-heap mechanism.
///
/// The mmap scheme is:
///
/// base ==> (points here)
/// - (image.offset bytes)   anonymous zero memory, pre-image
/// - (image.len bytes)      CoW mapping of memfd heap image
/// - (up to static_size)    anonymous zero memory, post-image
///
/// The ordering of mmaps to set this up is:
///
/// - once, when pooling allocator is created:
///   - one large mmap to create 8GiB * instances * memories slots
///
/// - per instantiation of new image in a slot:
///   - mmap of anonymous zero memory, from 0 to max heap size
///     (static_size)
///   - mmap of CoW'd memfd image, from `image.offset` to
///     `image.offset + image.len`. This overwrites part of the
///     anonymous zero memory, potentially splitting it into a pre-
///     and post-region.
///   - mprotect(PROT_NONE) on the part of the heap beyond the initial
///     heap size; we re-mprotect it with R+W bits when the heap is
///     grown.
#[derive(Debug)]
pub struct MemFdSlot {
    /// The base of the actual heap memory. Bytes at this address are
    /// what is seen by the Wasm guest code.
    base: usize,
    /// The maximum static memory size, plus post-guard.
    static_size: usize,
    /// The memfd image that backs this memory. May be `None`, in
    /// which case the memory is all zeroes.
    pub(crate) image: Option<Arc<MemoryMemFd>>,
    /// The initial heap size.
    initial_size: usize,
    /// The current heap size. All memory above `base + cur_size`
    /// should be PROT_NONE (mapped inaccessible).
    cur_size: usize,
    /// Whether this slot may have "dirty" pages (pages written by an
    /// instantiation). Set by `instantiate()` and cleared by
    /// `clear_and_remain_ready()`, and used in assertions to ensure
    /// those methods are called properly.
    dirty: bool,
}

impl MemFdSlot {
    pub(crate) fn create(base_addr: *mut c_void, static_size: usize) -> Self {
        let base = base_addr as usize;
        MemFdSlot {
            base,
            static_size,
            initial_size: 0,
            cur_size: 0,
            image: None,
            dirty: false,
        }
    }

    pub(crate) fn set_heap_limit(&mut self, size_bytes: usize) -> Result<()> {
        assert!(size_bytes > self.cur_size);
        // mprotect the relevant region.
        let start = self.base + self.cur_size;
        let len = size_bytes - self.cur_size;
        unsafe {
            rustix::io::mprotect(
                start as *mut _,
                len,
                rustix::io::MprotectFlags::READ | rustix::io::MprotectFlags::WRITE,
            )?;
        }

        Ok(())
    }

    pub(crate) fn instantiate(
        &mut self,
        initial_size_bytes: usize,
        maybe_image: Option<&Arc<MemoryMemFd>>,
    ) -> Result<(), InstantiationError> {
        assert!(!self.dirty);

        // Fast-path: previously instantiated with the same image, or
        // no image but the same initial size, so the mappings are
        // already correct; there is no need to mmap anything. Given
        // that we asserted not-dirty above, any dirty pages will have
        // already been thrown away by madvise() during the previous
        // termination.  The `clear_and_remain_ready()` path also
        // mprotects memory above the initial heap size back to
        // PROT_NONE, so we don't need to do that here.
        if (self.image.is_none()
            && maybe_image.is_none()
            && self.initial_size == initial_size_bytes)
            || (self.image.is_some()
                && maybe_image.is_some()
                && self.image.as_ref().unwrap().fd.as_file().as_raw_fd()
                    == maybe_image.as_ref().unwrap().fd.as_file().as_raw_fd())
        {
            self.dirty = true;
            return Ok(());
        }

        // Otherwise, we need to redo (i) the anonymous-mmap backing
        // for the whole slot, (ii) the initial-heap-image mapping if
        // present, and (iii) the mprotect(PROT_NONE) above the
        // initial heap size.

        // Security/audit note: we map all of these MAP_PRIVATE, so
        // all instance data is local to the mapping, not propagated
        // to the backing fd. We throw away this CoW overlay with
        // madvise() below, from base up to static_size (which is the
        // whole slot) when terminating the instance.

        // Anonymous mapping behind the initial heap size: this gives
        // zeroes for any "holes" in the initial heap image. Anonymous
        // mmap memory is faster to fault in than a CoW of a file,
        // even a file with zero holes, because the kernel's CoW path
        // unconditionally copies *something* (even if just a page of
        // zeroes). Anonymous zero pages are fast: the kernel
        // pre-zeroes them, and even if it runs out of those, a memset
        // is half as expensive as a memcpy (only writes, no reads).
        if initial_size_bytes > 0 {
            unsafe {
                let ptr = rustix::io::mmap_anonymous(
                    self.base as *mut c_void,
                    self.static_size,
                    rustix::io::ProtFlags::READ | rustix::io::ProtFlags::WRITE,
                    rustix::io::MapFlags::PRIVATE | rustix::io::MapFlags::FIXED,
                )
                .map_err(|e| InstantiationError::Resource(e.into()))?;
                assert_eq!(ptr as usize, self.base);
            }
        }

        // The initial memory image, if given. If not, we just get a
        // memory filled with zeroes.
        if let Some(image) = maybe_image {
            if image.len > 0 {
                let image = image.clone();

                unsafe {
                    let fd = rustix::fd::BorrowedFd::borrow_raw_fd(image.fd.as_file().as_raw_fd());
                    let ptr = rustix::io::mmap(
                        (self.base + image.offset) as *mut c_void,
                        image.len,
                        rustix::io::ProtFlags::READ | rustix::io::ProtFlags::WRITE,
                        rustix::io::MapFlags::PRIVATE | rustix::io::MapFlags::FIXED,
                        &fd,
                        image.offset as u64,
                    )
                    .map_err(|e| InstantiationError::Resource(e.into()))?;
                    assert_eq!(ptr as usize, self.base + image.offset);
                }

                self.image = Some(image);
            }
        }

        // mprotect above `initial_size_bytes`.
        self.initial_size = initial_size_bytes;
        self.protect_past_initial_size()
            .map_err(|e| InstantiationError::Resource(e.into()))?;

        self.dirty = true;
        Ok(())
    }

    pub(crate) fn clear_and_remain_ready(&mut self) -> Result<()> {
        assert!(self.dirty);
        // madvise the image range. This will throw away dirty pages,
        // which are CoW-private pages on top of the initial heap
        // image memfd.
        unsafe {
            rustix::io::madvise(
                self.base as *mut c_void,
                self.static_size,
                rustix::io::Advice::LinuxDontNeed,
            )?;
        }

        // mprotect the region beyond the initial heap size back to PROT_NONE.
        self.protect_past_initial_size()?;
        self.dirty = false;
        Ok(())
    }

    fn protect_past_initial_size(&self) -> Result<()> {
        let mprotect_start = self.base + self.initial_size;
        let mprotect_len = self.static_size - self.initial_size;
        if mprotect_len > 0 {
            unsafe {
                rustix::io::mprotect(
                    mprotect_start as *mut _,
                    mprotect_len,
                    rustix::io::MprotectFlags::empty(),
                )?;
            }
        }

        Ok(())
    }

    pub(crate) fn has_image(&self) -> bool {
        self.image.is_some()
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }
}

#[cfg(feature = "memfd-allocator")]
impl Drop for MemFdSlot {
    fn drop(&mut self) {
        // The MemFdSlot may be dropped if there is an error during
        // instantiation: for example, if a memory-growth limiter
        // disallows a guest from having a memory of a certain size,
        // after we've already initialized the MemFdSlot.
        //
        // We need to return this region of the large pool mmap to a
        // safe state (with no module-specific mappings). The
        // MemFdSlot will not be returned to the MemoryPool, so a new
        // MemFdSlot will be created and overwrite the mappings anyway
        // on the slot's next use; but for safety and to avoid
        // resource leaks it's better not to have stale mappings to a
        // possibly-otherwise-dead module's image.
        //
        // To "wipe the slate clean", let's do a mmap of anonymous
        // memory over the whole region, with PROT_NONE. Note that we
        // *can't* simply munmap, because that leaves a hole in the
        // middle of the pooling allocator's big memory area that some
        // other random mmap may swoop in and take, to be trampled
        // over by the next MemFdSlot later.
        //
        // Since we're in drop(), we can't sanely return an error if
        // this mmap fails. Let's ignore the failure if so; the next
        // MemFdSlot to be created for this slot will try to overwrite
        // the existing stale mappings, and return a failure properly
        // if we still cannot map new memory.
        unsafe {
            let _ = rustix::io::mmap_anonymous(
                self.base as *mut _,
                self.static_size,
                rustix::io::ProtFlags::empty(),
                rustix::io::MapFlags::FIXED | rustix::io::MapFlags::NORESERVE,
            );
        }
    }
}
