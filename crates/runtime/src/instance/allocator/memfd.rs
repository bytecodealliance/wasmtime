//! memfd mapping logic for use by the pooling allocator.

use crate::memfd::MemoryMemFd;
use crate::InstantiationError;
use anyhow::Result;
use libc::c_void;
use rustix::fd::AsRawFd;
use std::convert::TryFrom;
use std::fs::File;
use std::sync::Arc;

/// A single slot handled by the memfd instance-heap mechanism.
///
/// The mmap scheme is:
///
/// base ==> (points here)
/// - (image.offset bytes)   anonymous zero memory, pre-image
/// - (image.len bytes)      CoW mapping of memfd heap image
/// - (up to extension_offset)  anonymous zero memory, post-image
/// - (up to static_size)    heap expansion region; CoW mapping of per-slot memfd
///
/// The ordering of mmaps to set this up is:
///
/// - once, when pooling allocator is created:
///   - one large mmap to create 8GiB * instances * memories slots
///
/// - per instantiation of new image in a slot:
///   - mmap of anonymous zero memory, from 0 to initial heap size
///   - mmap of CoW'd memfd image, from `image.offset` to
///     `image.offset + image.len`. This overwrites part of the
///     anonymous zero memory, potentially splitting it into a pre-
///     and post-region.
///   - mmap of CoW'd extension file, past the initial heap size up to
///     the end of the max memory size (just before the
///     post-guard). This is always adjacent to the above mmaps, but
///     does not overlap/overwrite them.
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
    /// The offset at which the "extension file", which is used to
    /// allow for efficient heap growth, is mapped. This is always
    /// immediately after the end of the initial memory size.
    extension_offset: usize,
    /// The anonymous memfd, owned by this slot, which we mmap in the
    /// area where the heap may grow during runtime. We use the
    /// ftruncate() syscall (invoked via `File::set_len()`) to set its
    /// size. We never write any data to it -- we CoW-map it so we can
    /// throw away dirty data on termination. Instead, we just use its
    /// size as a "watermark" that delineates the boundary between
    /// safe-to-access memory and SIGBUS-causing memory. (This works
    /// because one can mmap a file beyond its end, and is good
    /// because ftruncate does not take the process-wide lock that
    /// mmap and mprotect do.)
    extension_file: File,
    /// Whether this slot may have "dirty" pages (pages written by an
    /// instantiation). Set by `instantiate()` and cleared by
    /// `clear_and_remain_ready()`, and used in assertions to ensure
    /// those methods are called properly.
    dirty: bool,
}

impl MemFdSlot {
    pub(crate) fn create(
        base_addr: *mut c_void,
        static_size: usize,
    ) -> Result<Self, InstantiationError> {
        let base = base_addr as usize;

        // Create a MemFD for the memory growth first -- this covers
        // extended heap beyond the initial image.
        let extension_memfd = memfd::MemfdOptions::new()
            .allow_sealing(true)
            .create("wasm-anonymous-heap")
            .map_err(|e| InstantiationError::Resource(e.into()))?;
        // Seal the ability to write the extension file (make it
        // permanently read-only). This is a defense-in-depth
        // mitigation to make extra-sure that we don't leak
        // information between instantiations. See note in `memfd.rs`
        // for more about why we use seals.
        extension_memfd
            .add_seal(memfd::FileSeal::SealWrite)
            .map_err(|e| InstantiationError::Resource(e.into()))?;
        extension_memfd
            .add_seal(memfd::FileSeal::SealSeal)
            .map_err(|e| InstantiationError::Resource(e.into()))?;
        let extension_file = extension_memfd.into_file();
        extension_file
            .set_len(0)
            .map_err(|e| InstantiationError::Resource(e.into()))?;

        Ok(MemFdSlot {
            base,
            static_size,
            image: None,
            extension_file,
            extension_offset: 0,
            dirty: false,
        })
    }

    pub(crate) fn set_heap_limit(&mut self, size_bytes: usize) -> Result<()> {
        assert!(size_bytes >= self.extension_offset);
        // This is all that is needed to make the new memory
        // accessible; we don't need to mprotect anything. (The
        // mapping itself is always R+W for the max possible heap
        // size, and only the anonymous-backing file length catches
        // out-of-bounds accesses.)
        self.extension_file
            .set_len(u64::try_from(size_bytes - self.extension_offset).unwrap())?;
        Ok(())
    }

    pub(crate) fn instantiate(
        &mut self,
        initial_size_bytes: usize,
        maybe_image: Option<&Arc<MemoryMemFd>>,
    ) -> Result<(), InstantiationError> {
        assert!(!self.dirty);

        if let Some(existing_image) = &self.image {
            // Fast-path: previously instantiated with the same image,
            // so the mappings are already correct; there is no need
            // to mmap anything. Given that we asserted not-dirty
            // above, any dirty pages will have already been thrown
            // away by madvise() during the previous termination.
            if let Some(image) = maybe_image {
                if existing_image.fd.as_file().as_raw_fd() == image.fd.as_file().as_raw_fd() {
                    self.dirty = true;
                    return Ok(());
                }
            }
        }

        // Otherwise, we need to redo (i) the anonymous-mmap backing
        // for the initial heap size, (ii) the extension-file backing,
        // and (iii) the initial-heap-image mapping if present.

        // Security/audit note: we map all of these MAP_PRIVATE, so
        // all instance data is local to the mapping, not propagated
        // to the backing fd. We throw away this CoW overlay with
        // madvise() below, from base up to extension_offset (which is
        // at least initial_size_bytes, and extended when the
        // extension file is, so it covers all three mappings) when
        // terminating the instance.

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
                    initial_size_bytes,
                    rustix::io::ProtFlags::READ | rustix::io::ProtFlags::WRITE,
                    rustix::io::MapFlags::PRIVATE | rustix::io::MapFlags::FIXED,
                )
                .map_err(|e| InstantiationError::Resource(e.into()))?;
                assert_eq!(ptr as usize, self.base);
            }
        }

        // An "extension file": this allows us to grow the heap by
        // doing just an ftruncate(), without changing any
        // mappings. This is important to avoid the process-wide mmap
        // lock on Linux.
        self.extension_offset = initial_size_bytes;
        let extension_map_len = self.static_size - initial_size_bytes;
        if extension_map_len > 0 {
            unsafe {
                let fd = rustix::fd::BorrowedFd::borrow_raw_fd(self.extension_file.as_raw_fd());
                let ptr = rustix::io::mmap(
                    (self.base + initial_size_bytes) as *mut c_void,
                    extension_map_len,
                    rustix::io::ProtFlags::READ | rustix::io::ProtFlags::WRITE,
                    rustix::io::MapFlags::PRIVATE | rustix::io::MapFlags::FIXED,
                    &fd,
                    0,
                )
                .map_err(|e| InstantiationError::Resource(e.into()))?;
                assert_eq!(ptr as usize, self.base + initial_size_bytes);
            }
        }

        // Finally, the initial memory image.
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

        self.dirty = true;
        Ok(())
    }

    pub(crate) fn clear_and_remain_ready(&mut self) -> Result<()> {
        assert!(self.dirty);
        // madvise the image range; that's it! This will throw away
        // dirty pages, which are CoW-private pages on top of the
        // initial heap image memfd.
        unsafe {
            rustix::io::madvise(
                self.base as *mut c_void,
                self.extension_offset,
                rustix::io::Advice::LinuxDontNeed,
            )?;
        }

        // truncate the extension file down to zero bytes to reset heap length.
        self.extension_file
            .set_len(0)
            .map_err(|e| InstantiationError::Resource(e.into()))?;
        self.dirty = false;
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
