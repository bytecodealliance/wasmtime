//! memfd support: creation of backing images for modules, and logic
//! to support mapping these backing images into memory.

use crate::InstantiationError;
use anyhow::Result;
use libc::c_void;
use memfd::{Memfd, MemfdOptions};
use rustix::fd::AsRawFd;
use rustix::fs::FileExt;
use std::sync::Arc;
use std::{convert::TryFrom, ops::Range};
use wasmtime_environ::{
    DefinedMemoryIndex, MemoryInitialization, MemoryInitializer, MemoryPlan, Module, PrimaryMap,
};

/// MemFDs containing backing images for certain memories in a module.
///
/// This is meant to be built once, when a module is first
/// loaded/constructed, and then used many times for instantiation.
pub struct ModuleMemFds {
    memories: PrimaryMap<DefinedMemoryIndex, Option<Arc<MemoryMemFd>>>,
}

const MAX_MEMFD_IMAGE_SIZE: u64 = 1024 * 1024 * 1024; // limit to 1GiB.

impl ModuleMemFds {
    pub(crate) fn get_memory_image(
        &self,
        defined_index: DefinedMemoryIndex,
    ) -> Option<&Arc<MemoryMemFd>> {
        self.memories[defined_index].as_ref()
    }
}

/// One backing image for one memory.
#[derive(Debug)]
pub struct MemoryMemFd {
    /// The actual memfd image: an anonymous file in memory which we
    /// use as the backing content for a copy-on-write (CoW) mapping
    /// in the memory region.
    pub fd: Memfd,
    /// Length of image. Note that initial memory size may be larger;
    /// leading and trailing zeroes are truncated (handled by
    /// anonymous backing memfd).
    ///
    /// Must be a multiple of the system page size.
    pub len: usize,
    /// Image starts this many bytes into heap space. Note that the
    /// memfd's offsets are always equal to the heap offsets, so we
    /// map at an offset into the fd as well. (This simplifies
    /// construction.)
    ///
    /// Must be a multiple of the system page size.
    pub offset: usize,
}

fn unsupported_initializer(segment: &MemoryInitializer, plan: &MemoryPlan) -> bool {
    // If the segment has a base that is dynamically determined
    // (by a global value, which may be a function of an imported
    // module, for example), then we cannot build a single static
    // image that is used for every instantiation. So we skip this
    // memory entirely.
    let end = match segment.end() {
        None => {
            return true;
        }
        Some(end) => end,
    };

    // Cannot be out-of-bounds. If there is a *possibility* it may
    // be, then we just fall back on ordinary initialization.
    if plan.initializer_possibly_out_of_bounds(segment) {
        return true;
    }

    // Must fit in our max size.
    if end > MAX_MEMFD_IMAGE_SIZE {
        return true;
    }

    false
}

fn create_memfd() -> Result<Memfd> {
    // Create the memfd. It needs a name, but the
    // documentation for `memfd_create()` says that names can
    // be duplicated with no issues.
    MemfdOptions::new()
        .allow_sealing(true)
        .create("wasm-memory-image")
        .map_err(|e| e.into())
}

impl ModuleMemFds {
    /// Create a new `ModuleMemFds` for the given module. This can be
    /// passed in as part of a `InstanceAllocationRequest` to speed up
    /// instantiation and execution by using memfd-backed memories.
    pub fn new(module: &Module, wasm_data: &[u8]) -> Result<Option<Arc<ModuleMemFds>>> {
        let page_size = region::page::size() as u64;
        let num_defined_memories = module.memory_plans.len() - module.num_imported_memories;

        // Allocate a memfd file initially for every memory. We'll
        // release those and set `excluded_memories` for those that we
        // determine during initializer processing we cannot support a
        // static image (e.g. due to dynamically-located segments).
        let mut memfds: PrimaryMap<DefinedMemoryIndex, Option<Memfd>> = PrimaryMap::default();
        let mut sizes: PrimaryMap<DefinedMemoryIndex, u64> = PrimaryMap::default();
        let mut excluded_memories: PrimaryMap<DefinedMemoryIndex, bool> = PrimaryMap::new();

        for _ in 0..num_defined_memories {
            memfds.push(None);
            sizes.push(0);
            excluded_memories.push(false);
        }

        let round_up_page = |len: u64| (len + page_size - 1) & !(page_size - 1);

        match &module.memory_initialization {
            &MemoryInitialization::Segmented(ref segments) => {
                for (i, segment) in segments.iter().enumerate() {
                    let defined_memory = match module.defined_memory_index(segment.memory_index) {
                        Some(defined_memory) => defined_memory,
                        None => continue,
                    };
                    if excluded_memories[defined_memory] {
                        continue;
                    }

                    if unsupported_initializer(segment, &module.memory_plans[segment.memory_index])
                    {
                        memfds[defined_memory] = None;
                        excluded_memories[defined_memory] = true;
                        continue;
                    }

                    if memfds[defined_memory].is_none() {
                        memfds[defined_memory] = Some(create_memfd()?);
                    }
                    let memfd = memfds[defined_memory].as_mut().unwrap();

                    let end = round_up_page(segment.end().expect("must have statically-known end"));
                    if end > sizes[defined_memory] {
                        sizes[defined_memory] = end;
                        memfd.as_file().set_len(end)?;
                    }

                    let base = segments[i].offset;
                    let data = &wasm_data[segment.data.start as usize..segment.data.end as usize];
                    memfd.as_file().write_at(data, base)?;
                }
            }
            &MemoryInitialization::Paged { ref map, .. } => {
                for (defined_memory, pages) in map {
                    let top = pages
                        .iter()
                        .map(|(base, range)| *base + range.len() as u64)
                        .max()
                        .unwrap_or(0);

                    let memfd = create_memfd()?;
                    memfd.as_file().set_len(top)?;

                    for (base, range) in pages {
                        let data = &wasm_data[range.start as usize..range.end as usize];
                        memfd.as_file().write_at(data, *base)?;
                    }

                    memfds[defined_memory] = Some(memfd);
                    sizes[defined_memory] = top;
                }
            }
        }

        // Now finalize each memory.
        let mut memories: PrimaryMap<DefinedMemoryIndex, Option<Arc<MemoryMemFd>>> =
            PrimaryMap::default();
        for (defined_memory, maybe_memfd) in memfds {
            let memfd = match maybe_memfd {
                Some(memfd) => memfd,
                None => {
                    memories.push(None);
                    continue;
                }
            };
            let size = sizes[defined_memory];

            // Find leading and trailing zero data so that the mmap
            // can precisely map only the nonzero data; anon-mmap zero
            // memory is faster for anything that doesn't actually
            // have content.
            let mut page_data = vec![0; page_size as usize];
            let mut page_is_nonzero = |page| {
                let offset = page_size * page;
                memfd.as_file().read_at(&mut page_data[..], offset).unwrap();
                page_data.iter().any(|byte| *byte != 0)
            };
            let n_pages = size / page_size;

            let mut offset = 0;
            for page in 0..n_pages {
                if page_is_nonzero(page) {
                    break;
                }
                offset += page_size;
            }
            let len = if offset == size {
                0
            } else {
                let mut len = 0;
                for page in (0..n_pages).rev() {
                    if page_is_nonzero(page) {
                        len = (page + 1) * page_size - offset;
                        break;
                    }
                }
                len
            };

            // Seal the memfd's data and length.
            //
            // This is a defense-in-depth security mitigation. The
            // memfd will serve as the starting point for the heap of
            // every instance of this module. If anything were to
            // write to this, it could affect every execution. The
            // memfd object itself is owned by the machinery here and
            // not exposed elsewhere, but it is still an ambient open
            // file descriptor at the syscall level, so some other
            // vulnerability that allowed writes to arbitrary fds
            // could modify it. Or we could have some issue with the
            // way that we map it into each instance. To be
            // extra-super-sure that it never changes, and because
            // this costs very little, we use the kernel's "seal" API
            // to make the memfd image permanently read-only.
            memfd.add_seal(memfd::FileSeal::SealGrow)?;
            memfd.add_seal(memfd::FileSeal::SealShrink)?;
            memfd.add_seal(memfd::FileSeal::SealWrite)?;
            memfd.add_seal(memfd::FileSeal::SealSeal)?;

            assert_eq!(offset % page_size, 0);
            assert_eq!(len % page_size, 0);

            memories.push(Some(Arc::new(MemoryMemFd {
                fd: memfd,
                offset: usize::try_from(offset).unwrap(),
                len: usize::try_from(len).unwrap(),
            })));
        }

        Ok(Some(Arc::new(ModuleMemFds { memories })))
    }
}

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
    ///
    /// Invariant: if !dirty, then this memory slot contains a clean
    /// CoW mapping of `image`, if `Some(..)`, and anonymous-zero
    /// memory beyond the image up to `static_size`. The addresses
    /// from offset 0 to `initial_size` are accessible R+W and the
    /// rest of the slot is inaccessible.
    dirty: bool,
    /// Whether this MemFdSlot is responsible for mapping anonymous
    /// memory (to hold the reservation while overwriting mappings
    /// specific to this slot) in place when it is dropped. Default
    /// on, unless the caller knows what they are doing.
    clear_on_drop: bool,
}

impl MemFdSlot {
    /// Create a new MemFdSlot. Assumes that there is an anonymous
    /// mmap backing in the given range to start.
    pub(crate) fn create(base_addr: *mut c_void, initial_size: usize, static_size: usize) -> Self {
        let base = base_addr as usize;
        MemFdSlot {
            base,
            static_size,
            initial_size,
            cur_size: initial_size,
            image: None,
            dirty: false,
            clear_on_drop: true,
        }
    }

    /// Inform the MemFdSlot that it should *not* clear the underlying
    /// address space when dropped. This should be used only when the
    /// caller will clear or reuse the address space in some other
    /// way.
    pub(crate) fn no_clear_on_drop(&mut self) {
        self.clear_on_drop = false;
    }

    pub(crate) fn set_heap_limit(&mut self, size_bytes: usize) -> Result<()> {
        // mprotect the relevant region.
        self.set_protection(
            self.cur_size..size_bytes,
            rustix::io::MprotectFlags::READ | rustix::io::MprotectFlags::WRITE,
        )?;
        self.cur_size = size_bytes;

        Ok(())
    }

    pub(crate) fn instantiate(
        &mut self,
        initial_size_bytes: usize,
        maybe_image: Option<&Arc<MemoryMemFd>>,
    ) -> Result<(), InstantiationError> {
        assert!(!self.dirty);
        assert_eq!(self.cur_size, self.initial_size);

        // Fast-path: previously instantiated with the same image, or
        // no image but the same initial size, so the mappings are
        // already correct; there is no need to mmap anything. Given
        // that we asserted not-dirty above, any dirty pages will have
        // already been thrown away by madvise() during the previous
        // termination. The `clear_and_remain_ready()` path also
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
        // Otherwise, we need to transition from the previous state to the
        // state now requested. An attempt is made here to minimize syscalls to
        // the kernel to ideally reduce the overhead of this as it's fairly
        // performance sensitive with memories. Note that the "previous state"
        // is assumed to be post-initialization (e.g. after an mmap on-demand
        // memory was created) or after `clear_and_remain_ready` was called
        // which notably means that `madvise` has reset all the memory back to
        // its original state.
        //
        // Security/audit note: we map all of these MAP_PRIVATE, so
        // all instance data is local to the mapping, not propagated
        // to the backing fd. We throw away this CoW overlay with
        // madvise() below, from base up to static_size (which is the
        // whole slot) when terminating the instance.

        if self.image.is_some() {
            // In this case the state of memory at this time is that the memory
            // from `0..self.initial_size` is reset back to its original state,
            // but this memory contians a CoW image that is different from the
            // one specified here. To reset state we first reset the mapping
            // of memory to anonymous PROT_NONE memory, and then afterwards the
            // heap is made visible with an mprotect.
            self.reset_with_anon_memory()
                .map_err(|e| InstantiationError::Resource(e.into()))?;
            self.set_protection(
                0..initial_size_bytes,
                rustix::io::MprotectFlags::READ | rustix::io::MprotectFlags::WRITE,
            )
            .map_err(|e| InstantiationError::Resource(e.into()))?;
        } else if initial_size_bytes < self.initial_size {
            // In this case the previous module had now CoW image which means
            // that the memory at `0..self.initial_size` is all zeros and
            // read-write, everything afterwards being PROT_NONE.
            //
            // Our requested heap size is smaller than the previous heap size
            // so all that's needed now is to shrink the heap further to
            // `initial_size_bytes`.
            //
            // So we come in with:
            // - anon-zero memory, R+W,  [0, self.initial_size)
            // - anon-zero memory, none, [self.initial_size, self.static_size)
            // and we want:
            // - anon-zero memory, R+W,  [0, initial_size_bytes)
            // - anon-zero memory, none, [initial_size_bytes, self.static_size)
            //
            // so given initial_size_bytes < self.initial_size we
            // mprotect(NONE) the zone from the first to the second.
            self.set_protection(
                initial_size_bytes..self.initial_size,
                rustix::io::MprotectFlags::empty(),
            )
            .map_err(|e| InstantiationError::Resource(e.into()))?;
        } else if initial_size_bytes > self.initial_size {
            // In this case, like the previous one, the previous module had no
            // CoW image but had a smaller heap than desired for this module.
            // That means that here `mprotect` is used to make the new pages
            // read/write, and since they're all reset from before they'll be
            // made visible as zeros.
            self.set_protection(
                self.initial_size..initial_size_bytes,
                rustix::io::MprotectFlags::READ | rustix::io::MprotectFlags::WRITE,
            )
            .map_err(|e| InstantiationError::Resource(e.into()))?;
        } else {
            // The final case here is that the previous module has no CoW image
            // so the previous heap is all zeros. The previous heap is the exact
            // same size as the requested heap, so no syscalls are needed to do
            // anything else.
        }

        // The memory image, at this point, should have `initial_size_bytes` of
        // zeros starting at `self.base` followed by inaccessible memory to
        // `self.static_size`. Update sizing fields to reflect this.
        self.initial_size = initial_size_bytes;
        self.cur_size = initial_size_bytes;

        // The initial memory image, if given. If not, we just get a
        // memory filled with zeroes.
        if let Some(image) = maybe_image {
            assert!(image.offset.checked_add(image.len).unwrap() <= initial_size_bytes);
            if image.len > 0 {
                unsafe {
                    let ptr = rustix::io::mmap(
                        (self.base + image.offset) as *mut c_void,
                        image.len,
                        rustix::io::ProtFlags::READ | rustix::io::ProtFlags::WRITE,
                        rustix::io::MapFlags::PRIVATE | rustix::io::MapFlags::FIXED,
                        image.fd.as_file(),
                        image.offset as u64,
                    )
                    .map_err(|e| InstantiationError::Resource(e.into()))?;
                    assert_eq!(ptr as usize, self.base + image.offset);
                }
            }
        }

        self.image = maybe_image.cloned();
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
                self.cur_size,
                rustix::io::Advice::LinuxDontNeed,
            )?;
        }

        // mprotect the initial heap region beyond the initial heap size back to PROT_NONE.
        self.set_protection(
            self.initial_size..self.cur_size,
            rustix::io::MprotectFlags::empty(),
        )?;
        self.cur_size = self.initial_size;
        self.dirty = false;
        Ok(())
    }

    fn set_protection(&self, range: Range<usize>, flags: rustix::io::MprotectFlags) -> Result<()> {
        assert!(range.start <= range.end);
        assert!(range.end <= self.static_size);
        let mprotect_start = self.base.checked_add(range.start).unwrap();
        if range.len() > 0 {
            unsafe {
                rustix::io::mprotect(mprotect_start as *mut _, range.len(), flags)?;
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

    /// Map anonymous zeroed memory across the whole slot,
    /// inaccessible. Used both during instantiate and during drop.
    fn reset_with_anon_memory(&self) -> Result<()> {
        unsafe {
            let ptr = rustix::io::mmap_anonymous(
                self.base as *mut c_void,
                self.static_size,
                rustix::io::ProtFlags::empty(),
                rustix::io::MapFlags::PRIVATE | rustix::io::MapFlags::FIXED,
            )?;
            assert_eq!(ptr as usize, self.base);
        }
        Ok(())
    }
}

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
        //
        // The exception to all of this is if the `unmap_on_drop` flag
        // (which is set by default) is false. If so, the owner of
        // this MemFdSlot has indicated that it will clean up in some
        // other way.
        if self.clear_on_drop {
            let _ = self.reset_with_anon_memory();
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::create_memfd;
    use super::MemFdSlot;
    use super::MemoryMemFd;
    use crate::mmap::Mmap;
    use anyhow::Result;
    use rustix::fs::FileExt;

    fn create_memfd_with_data(offset: usize, data: &[u8]) -> Result<MemoryMemFd> {
        let page_size = region::page::size();
        let memfd = create_memfd()?;
        // Offset and length have to be page-aligned.
        assert_eq!(offset & (page_size - 1), 0);
        let image_len = offset + data.len();
        let image_len = (image_len + page_size - 1) & !(page_size - 1);
        memfd.as_file().set_len(image_len as u64)?;
        memfd.as_file().write_at(data, offset as u64)?;
        Ok(MemoryMemFd {
            fd: memfd,
            len: image_len,
            offset,
        })
    }

    #[test]
    fn instantiate_no_image() {
        if skip_tests_due_to_qemu_madvise_semantics() {
            return;
        }

        // 4 MiB mmap'd area, not accessible
        let mut mmap = Mmap::accessible_reserved(0, 4 << 20).unwrap();
        // Create a MemFdSlot on top of it
        let mut memfd = MemFdSlot::create(mmap.as_mut_ptr() as *mut _, 0, 4 << 20);
        memfd.no_clear_on_drop();
        assert!(!memfd.is_dirty());
        // instantiate with 64 KiB initial size
        memfd.instantiate(64 << 10, None).unwrap();
        assert!(memfd.is_dirty());
        // We should be able to access this 64 KiB (try both ends) and
        // it should consist of zeroes.
        let slice = mmap.as_mut_slice();
        assert_eq!(0, slice[0]);
        assert_eq!(0, slice[65535]);
        slice[1024] = 42;
        assert_eq!(42, slice[1024]);
        // grow the heap
        memfd.set_heap_limit(128 << 10).unwrap();
        let slice = mmap.as_slice();
        assert_eq!(42, slice[1024]);
        assert_eq!(0, slice[131071]);
        // instantiate again; we should see zeroes, even as the
        // reuse-anon-mmap-opt kicks in
        memfd.clear_and_remain_ready().unwrap();
        assert!(!memfd.is_dirty());
        memfd.instantiate(64 << 10, None).unwrap();
        let slice = mmap.as_slice();
        assert_eq!(0, slice[1024]);
    }

    #[test]
    fn instantiate_image() {
        if skip_tests_due_to_qemu_madvise_semantics() {
            return;
        }

        // 4 MiB mmap'd area, not accessible
        let mut mmap = Mmap::accessible_reserved(0, 4 << 20).unwrap();
        // Create a MemFdSlot on top of it
        let mut memfd = MemFdSlot::create(mmap.as_mut_ptr() as *mut _, 0, 4 << 20);
        memfd.no_clear_on_drop();
        // Create an image with some data.
        let image = Arc::new(create_memfd_with_data(4096, &[1, 2, 3, 4]).unwrap());
        // Instantiate with this image
        memfd.instantiate(64 << 10, Some(&image)).unwrap();
        assert!(memfd.has_image());
        let slice = mmap.as_mut_slice();
        assert_eq!(&[1, 2, 3, 4], &slice[4096..4100]);
        slice[4096] = 5;
        // Clear and re-instantiate same image
        memfd.clear_and_remain_ready().unwrap();
        memfd.instantiate(64 << 10, Some(&image)).unwrap();
        let slice = mmap.as_slice();
        // Should not see mutation from above
        assert_eq!(&[1, 2, 3, 4], &slice[4096..4100]);
        // Clear and re-instantiate no image
        memfd.clear_and_remain_ready().unwrap();
        memfd.instantiate(64 << 10, None).unwrap();
        assert!(!memfd.has_image());
        let slice = mmap.as_slice();
        assert_eq!(&[0, 0, 0, 0], &slice[4096..4100]);
        // Clear and re-instantiate image again
        memfd.clear_and_remain_ready().unwrap();
        memfd.instantiate(64 << 10, Some(&image)).unwrap();
        let slice = mmap.as_slice();
        assert_eq!(&[1, 2, 3, 4], &slice[4096..4100]);
        // Create another image with different data.
        let image2 = Arc::new(create_memfd_with_data(4096, &[10, 11, 12, 13]).unwrap());
        memfd.clear_and_remain_ready().unwrap();
        memfd.instantiate(128 << 10, Some(&image2)).unwrap();
        let slice = mmap.as_slice();
        assert_eq!(&[10, 11, 12, 13], &slice[4096..4100]);
        // Instantiate the original image again; we should notice it's
        // a different image and not reuse the mappings.
        memfd.clear_and_remain_ready().unwrap();
        memfd.instantiate(64 << 10, Some(&image)).unwrap();
        let slice = mmap.as_slice();
        assert_eq!(&[1, 2, 3, 4], &slice[4096..4100]);
    }

    /// qemu's madvise implementation does not implement the
    /// "flash-reset back to zero or CoW backing" semantics that Linux
    /// does. Our CI setup uses qemu (in usermode-binary mode, not
    /// whole-system mode) to run tests on aarch64 and s390x. We want
    /// to skip these tests when under qemu, but not when someone is
    /// developing natively on one of these architectures. So instead,
    /// we dynamically detect an environment variable that our CI
    /// setup sets.
    ///
    /// See `skip_pooling_allocator_tests()` in `tests/all/main.rs`
    /// for more.
    fn skip_tests_due_to_qemu_madvise_semantics() -> bool {
        std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok()
    }
}
