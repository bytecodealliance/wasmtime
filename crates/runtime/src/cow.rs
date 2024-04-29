//! Copy-on-write initialization support: creation of backing images for
//! modules, and logic to support mapping these backing images into memory.

use crate::sys::vm::{self, MemoryImageSource};
use crate::{MmapVec, SendSyncPtr};
use anyhow::Result;
use std::ffi::c_void;
use std::ops::Range;
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::{
    DefinedMemoryIndex, MemoryInitialization, MemoryPlan, MemoryStyle, Module, PrimaryMap,
};

/// Backing images for memories in a module.
///
/// This is meant to be built once, when a module is first loaded/constructed,
/// and then used many times for instantiation.
pub struct ModuleMemoryImages {
    memories: PrimaryMap<DefinedMemoryIndex, Option<Arc<MemoryImage>>>,
}

impl ModuleMemoryImages {
    /// Get the MemoryImage for a given memory.
    pub fn get_memory_image(&self, defined_index: DefinedMemoryIndex) -> Option<&Arc<MemoryImage>> {
        self.memories[defined_index].as_ref()
    }
}

/// One backing image for one memory.
#[derive(Debug, PartialEq)]
pub struct MemoryImage {
    /// The platform-specific source of this image.
    ///
    /// This might be a mapped `*.cwasm` file or on Unix it could also be a
    /// `Memfd` as an anonymous file in memory on Linux. In either case this is
    /// used as the backing-source for the CoW image.
    source: MemoryImageSource,

    /// Length of image, in bytes.
    ///
    /// Note that initial memory size may be larger; leading and trailing zeroes
    /// are truncated (handled by backing fd).
    ///
    /// Must be a multiple of the system page size.
    len: usize,

    /// Image starts this many bytes into `source`.
    ///
    /// This is 0 for anonymous-backed memfd files and is the offset of the
    /// data section in a `*.cwasm` file for `*.cwasm`-backed images.
    ///
    /// Must be a multiple of the system page size.
    source_offset: u64,

    /// Image starts this many bytes into heap space.
    ///
    /// Must be a multiple of the system page size.
    linear_memory_offset: usize,
}

impl MemoryImage {
    fn new(
        page_size: u32,
        offset: u64,
        data: &[u8],
        mmap: Option<&MmapVec>,
    ) -> Result<Option<MemoryImage>> {
        // Sanity-check that various parameters are page-aligned.
        let len = data.len();
        assert_eq!(offset % u64::from(page_size), 0);
        assert_eq!((len as u32) % page_size, 0);
        let linear_memory_offset = match usize::try_from(offset) {
            Ok(offset) => offset,
            Err(_) => return Ok(None),
        };

        // If a backing `mmap` is present then `data` should be a sub-slice of
        // the `mmap`. The sanity-checks here double-check that. Additionally
        // compilation should have ensured that the `data` section is
        // page-aligned within `mmap`, so that's also all double-checked here.
        //
        // Finally if the `mmap` itself comes from a backing file on disk, such
        // as a `*.cwasm` file, then that's a valid source of data for the
        // memory image so we simply return referencing that.
        //
        // Note that this path is platform-agnostic in the sense of all
        // platforms we support support memory mapping copy-on-write data from
        // files, but for now this is still a Linux-specific region of Wasmtime.
        // Some work will be needed to get this file compiling for macOS and
        // Windows.
        if let Some(mmap) = mmap {
            let start = mmap.as_ptr() as usize;
            let end = start + mmap.len();
            let data_start = data.as_ptr() as usize;
            let data_end = data_start + data.len();
            assert!(start <= data_start && data_end <= end);
            assert_eq!((start as u32) % page_size, 0);
            assert_eq!((data_start as u32) % page_size, 0);
            assert_eq!((data_end as u32) % page_size, 0);
            assert_eq!((mmap.original_offset() as u32) % page_size, 0);

            if let Some(file) = mmap.original_file() {
                if let Some(source) = MemoryImageSource::from_file(file) {
                    return Ok(Some(MemoryImage {
                        source,
                        source_offset: u64::try_from(mmap.original_offset() + (data_start - start))
                            .unwrap(),
                        linear_memory_offset,
                        len,
                    }));
                }
            }
        }

        // If `mmap` doesn't come from a file then platform-specific mechanisms
        // may be used to place the data in a form that's amenable to an mmap.
        if let Some(source) = MemoryImageSource::from_data(data)? {
            return Ok(Some(MemoryImage {
                source,
                source_offset: 0,
                linear_memory_offset,
                len,
            }));
        }

        Ok(None)
    }

    unsafe fn map_at(&self, base: *mut u8) -> Result<()> {
        self.source.map_at(
            base.add(self.linear_memory_offset),
            self.len,
            self.source_offset,
        )?;
        Ok(())
    }

    unsafe fn remap_as_zeros_at(&self, base: *mut u8) -> Result<()> {
        self.source
            .remap_as_zeros_at(base.add(self.linear_memory_offset), self.len)?;
        Ok(())
    }
}

impl ModuleMemoryImages {
    /// Create a new `ModuleMemoryImages` for the given module. This can be
    /// passed in as part of a `InstanceAllocationRequest` to speed up
    /// instantiation and execution by using copy-on-write-backed memories.
    pub fn new(
        module: &Module,
        wasm_data: &[u8],
        mmap: Option<&MmapVec>,
    ) -> Result<Option<ModuleMemoryImages>> {
        let map = match &module.memory_initialization {
            MemoryInitialization::Static { map } => map,
            _ => return Ok(None),
        };
        let mut memories = PrimaryMap::with_capacity(map.len());
        let page_size = crate::page_size() as u32;
        for (memory_index, init) in map {
            // mmap-based-initialization only works for defined memories with a
            // known starting point of all zeros, so bail out if the mmeory is
            // imported.
            let defined_memory = match module.defined_memory_index(memory_index) {
                Some(idx) => idx,
                None => return Ok(None),
            };

            // If there's no initialization for this memory known then we don't
            // need an image for the memory so push `None` and move on.
            let init = match init {
                Some(init) => init,
                None => {
                    memories.push(None);
                    continue;
                }
            };

            // Get the image for this wasm module  as a subslice of `wasm_data`,
            // and then use that to try to create the `MemoryImage`. If this
            // creation files then we fail creating `ModuleMemoryImages` since this
            // memory couldn't be represented.
            let data = &wasm_data[init.data.start as usize..init.data.end as usize];
            let image = match MemoryImage::new(page_size, init.offset, data, mmap)? {
                Some(image) => image,
                None => return Ok(None),
            };

            let idx = memories.push(Some(Arc::new(image)));
            assert_eq!(idx, defined_memory);
        }

        Ok(Some(ModuleMemoryImages { memories }))
    }
}

/// Slot management of a copy-on-write image which can be reused for the pooling
/// allocator.
///
/// This data structure manages a slot of linear memory, primarily in the
/// pooling allocator, which optionally has a contiguous memory image in the
/// middle of it. Pictorially this data structure manages a virtual memory
/// region that looks like:
///
/// ```text
///   +--------------------+-------------------+--------------+--------------+
///   |   anonymous        |      optional     |   anonymous  |    PROT_NONE |
///   |     zero           |       memory      |     zero     |     memory   |
///   |    memory          |       image       |    memory    |              |
///   +--------------------+-------------------+--------------+--------------+
///   |                     <------+---------->
///   |<-----+------------>         \
///   |      \                   image.len
///   |       \
///   |  image.linear_memory_offset
///   |
///   \
///  self.base is this virtual address
///
///    <------------------+------------------------------------------------>
///                        \
///                      static_size
///
///    <------------------+---------------------------------->
///                        \
///                      accessible
/// ```
///
/// When a `MemoryImageSlot` is created it's told what the `static_size` and
/// `accessible` limits are. Initially there is assumed to be no image in linear
/// memory.
///
/// When `MemoryImageSlot::instantiate` is called then the method will perform
/// a "synchronization" to take the image from its prior state to the new state
/// for the image specified. The first instantiation for example will mmap the
/// heap image into place. Upon reuse of a slot nothing happens except possibly
/// shrinking `self.accessible`. When a new image is used then the old image is
/// mapped to anonymous zero memory and then the new image is mapped in place.
///
/// A `MemoryImageSlot` is either `dirty` or it isn't. When a `MemoryImageSlot`
/// is dirty then it is assumed that any memory beneath `self.accessible` could
/// have any value. Instantiation cannot happen into a `dirty` slot, however, so
/// the `MemoryImageSlot::clear_and_remain_ready` returns this memory back to
/// its original state to mark `dirty = false`. This is done by resetting all
/// anonymous memory back to zero and the image itself back to its initial
/// contents.
///
/// On Linux this is achieved with the `madvise(MADV_DONTNEED)` syscall. This
/// syscall will release the physical pages back to the OS but retain the
/// original mappings, effectively resetting everything back to its initial
/// state. Non-linux platforms will replace all memory below `self.accessible`
/// with a fresh zero'd mmap, meaning that reuse is effectively not supported.
#[derive(Debug)]
pub struct MemoryImageSlot {
    /// The base address in virtual memory of the actual heap memory.
    ///
    /// Bytes at this address are what is seen by the Wasm guest code.
    base: SendSyncPtr<u8>,

    /// The maximum static memory size which `self.accessible` can grow to.
    static_size: usize,

    /// An optional image that is currently being used in this linear memory.
    ///
    /// This can be `None` in which case memory is originally all zeros. When
    /// `Some` the image describes where it's located within the image.
    image: Option<Arc<MemoryImage>>,

    /// The size of the heap that is readable and writable.
    ///
    /// Note that this may extend beyond the actual linear memory heap size in
    /// the case of dynamic memories in use. Memory accesses to memory below
    /// `self.accessible` may still page fault as pages are lazily brought in
    /// but the faults will always be resolved by the kernel.
    accessible: usize,

    /// Whether this slot may have "dirty" pages (pages written by an
    /// instantiation). Set by `instantiate()` and cleared by
    /// `clear_and_remain_ready()`, and used in assertions to ensure
    /// those methods are called properly.
    ///
    /// Invariant: if !dirty, then this memory slot contains a clean
    /// CoW mapping of `image`, if `Some(..)`, and anonymous-zero
    /// memory beyond the image up to `static_size`. The addresses
    /// from offset 0 to `self.accessible` are R+W and set to zero or the
    /// initial image content, as appropriate. Everything between
    /// `self.accessible` and `self.static_size` is inaccessible.
    dirty: bool,

    /// Whether this MemoryImageSlot is responsible for mapping anonymous
    /// memory (to hold the reservation while overwriting mappings
    /// specific to this slot) in place when it is dropped. Default
    /// on, unless the caller knows what they are doing.
    clear_on_drop: bool,
}

impl MemoryImageSlot {
    /// Create a new MemoryImageSlot. Assumes that there is an anonymous
    /// mmap backing in the given range to start.
    ///
    /// The `accessible` parameter describes how much of linear memory is
    /// already mapped as R/W with all zero-bytes. The `static_size` value is
    /// the maximum size of this image which `accessible` cannot grow beyond,
    /// and all memory from `accessible` from `static_size` should be mapped as
    /// `PROT_NONE` backed by zero-bytes.
    pub(crate) fn create(base_addr: *mut c_void, accessible: usize, static_size: usize) -> Self {
        MemoryImageSlot {
            base: NonNull::new(base_addr.cast()).unwrap().into(),
            static_size,
            accessible,
            image: None,
            dirty: false,
            clear_on_drop: true,
        }
    }

    #[cfg(feature = "pooling-allocator")]
    pub(crate) fn dummy() -> MemoryImageSlot {
        MemoryImageSlot {
            // This pointer isn't ever actually used so its value doesn't
            // matter but we need to satisfy `NonNull` requirement so create a
            // `dangling` pointer as a sentinel that should cause problems if
            // it's actually used.
            base: NonNull::dangling().into(),
            static_size: 0,
            image: None,
            accessible: 0,
            dirty: false,
            clear_on_drop: false,
        }
    }

    /// Inform the MemoryImageSlot that it should *not* clear the underlying
    /// address space when dropped. This should be used only when the
    /// caller will clear or reuse the address space in some other
    /// way.
    pub(crate) fn no_clear_on_drop(&mut self) {
        self.clear_on_drop = false;
    }

    pub(crate) fn set_heap_limit(&mut self, size_bytes: usize) -> Result<()> {
        assert!(size_bytes <= self.static_size);

        // If the heap limit already addresses accessible bytes then no syscalls
        // are necessary since the data is already mapped into the process and
        // waiting to go.
        //
        // This is used for "dynamic" memories where memory is not always
        // decommitted during recycling (but it's still always reset).
        if size_bytes <= self.accessible {
            return Ok(());
        }

        // Otherwise use `mprotect` to make the new pages read/write.
        self.set_protection(self.accessible..size_bytes, true)?;
        self.accessible = size_bytes;

        Ok(())
    }

    /// Prepares this slot for the instantiation of a new instance with the
    /// provided linear memory image.
    ///
    /// The `initial_size_bytes` parameter indicates the required initial size
    /// of the heap for the instance. The `maybe_image` is an optional initial
    /// image for linear memory to contains. The `style` is the way compiled
    /// code will be accessing this memory.
    ///
    /// The purpose of this method is to take a previously pristine slot
    /// (`!self.dirty`) and transform its prior state into state necessary for
    /// the given parameters. This could include, for example:
    ///
    /// * More memory may be made read/write if `initial_size_bytes` is larger
    ///   than `self.accessible`.
    /// * For `MemoryStyle::Static` linear memory may be made `PROT_NONE` if
    ///   `self.accessible` is larger than `initial_size_bytes`.
    /// * If no image was previously in place or if the wrong image was
    ///   previously in place then `mmap` may be used to setup the initial
    ///   image.
    pub(crate) fn instantiate(
        &mut self,
        initial_size_bytes: usize,
        maybe_image: Option<&Arc<MemoryImage>>,
        plan: &MemoryPlan,
    ) -> Result<()> {
        assert!(!self.dirty);
        assert!(initial_size_bytes <= self.static_size);

        // First order of business is to blow away the previous linear memory
        // image if it doesn't match the image specified here. If one is
        // detected then it's reset with anonymous memory which means that all
        // of memory up to `self.accessible` will now be read/write and zero.
        //
        // Note that this intentionally a "small mmap" which only covers the
        // extent of the prior initialization image in order to preserve
        // resident memory that might come before or after the image.
        if self.image.as_ref() != maybe_image {
            self.remove_image()?;
        }

        // The next order of business is to ensure that `self.accessible` is
        // appropriate. First up is to grow the read/write portion of memory if
        // it's not large enough to accommodate `initial_size_bytes`.
        if self.accessible < initial_size_bytes {
            self.set_protection(self.accessible..initial_size_bytes, true)?;
            self.accessible = initial_size_bytes;
        }

        // If (1) the accessible region is not in its initial state, and (2) the
        // memory relies on virtual memory at all (i.e. has offset guard pages
        // and/or is static), then we need to reset memory protections. Put
        // another way, the only time it is safe to not reset protections is
        // when we are using dynamic memory without any guard pages.
        if initial_size_bytes < self.accessible
            && (plan.offset_guard_size > 0 || matches!(plan.style, MemoryStyle::Static { .. }))
        {
            self.set_protection(initial_size_bytes..self.accessible, false)?;
            self.accessible = initial_size_bytes;
        }

        // Now that memory is sized appropriately the final operation is to
        // place the new image into linear memory. Note that this operation is
        // skipped if `self.image` matches `maybe_image`.
        assert!(initial_size_bytes <= self.accessible);
        if self.image.as_ref() != maybe_image {
            if let Some(image) = maybe_image.as_ref() {
                assert!(
                    image.linear_memory_offset.checked_add(image.len).unwrap()
                        <= initial_size_bytes
                );
                if image.len > 0 {
                    unsafe {
                        image.map_at(self.base.as_ptr())?;
                    }
                }
            }
            self.image = maybe_image.cloned();
        }

        // Flag ourselves as `dirty` which means that the next operation on this
        // slot is required to be `clear_and_remain_ready`.
        self.dirty = true;

        Ok(())
    }

    pub(crate) fn remove_image(&mut self) -> Result<()> {
        if let Some(image) = &self.image {
            unsafe {
                image.remap_as_zeros_at(self.base.as_ptr())?;
            }
            self.image = None;
        }
        Ok(())
    }

    /// Resets this linear memory slot back to a "pristine state".
    ///
    /// This will reset the memory back to its original contents on Linux or
    /// reset the contents back to zero on other platforms. The `keep_resident`
    /// argument is the maximum amount of memory to keep resident in this
    /// process's memory on Linux. Up to that much memory will be `memset` to
    /// zero where the rest of it will be reset or released with `madvise`.
    #[allow(dead_code)] // ignore warnings as this is only used in some cfgs
    pub(crate) fn clear_and_remain_ready(&mut self, keep_resident: usize) -> Result<()> {
        assert!(self.dirty);

        unsafe {
            self.reset_all_memory_contents(keep_resident)?;
        }

        self.dirty = false;
        Ok(())
    }

    #[allow(dead_code)] // ignore warnings as this is only used in some cfgs
    unsafe fn reset_all_memory_contents(&mut self, keep_resident: usize) -> Result<()> {
        if !vm::supports_madvise_dontneed() {
            // If we're not on Linux then there's no generic platform way to
            // reset memory back to its original state, so instead reset memory
            // back to entirely zeros with an anonymous backing.
            //
            // Additionally the previous image, if any, is dropped here
            // since it's no longer applicable to this mapping.
            return self.reset_with_anon_memory();
        }

        match &self.image {
            Some(image) => {
                assert!(self.accessible >= image.linear_memory_offset + image.len);
                if image.linear_memory_offset < keep_resident {
                    // If the image starts below the `keep_resident` then
                    // memory looks something like this:
                    //
                    //               up to `keep_resident` bytes
                    //                          |
                    //          +--------------------------+  remaining_memset
                    //          |                          | /
                    //  <-------------->                <------->
                    //
                    //                              image_end
                    // 0        linear_memory_offset   |             accessible
                    // |                |              |                  |
                    // +----------------+--------------+---------+--------+
                    // |  dirty memory  |    image     |   dirty memory   |
                    // +----------------+--------------+---------+--------+
                    //
                    //  <------+-------> <-----+----->  <---+---> <--+--->
                    //         |               |            |        |
                    //         |               |            |        |
                    //   memset (1)            /            |   madvise (4)
                    //                  mmadvise (2)       /
                    //                                    /
                    //                              memset (3)
                    //
                    //
                    // In this situation there are two disjoint regions that are
                    // `memset` manually to zero. Note that `memset (3)` may be
                    // zero bytes large. Furthermore `madvise (4)` may also be
                    // zero bytes large.

                    let image_end = image.linear_memory_offset + image.len;
                    let mem_after_image = self.accessible - image_end;
                    let remaining_memset =
                        (keep_resident - image.linear_memory_offset).min(mem_after_image);

                    // This is memset (1)
                    std::ptr::write_bytes(self.base.as_ptr(), 0u8, image.linear_memory_offset);

                    // This is madvise (2)
                    self.madvise_reset(image.linear_memory_offset, image.len)?;

                    // This is memset (3)
                    std::ptr::write_bytes(self.base.as_ptr().add(image_end), 0u8, remaining_memset);

                    // This is madvise (4)
                    self.madvise_reset(
                        image_end + remaining_memset,
                        mem_after_image - remaining_memset,
                    )?;
                } else {
                    // If the image starts after the `keep_resident` threshold
                    // then we memset the start of linear memory and then use
                    // madvise below for the rest of it, including the image.
                    //
                    // 0             keep_resident                   accessible
                    // |                |                                 |
                    // +----------------+---+----------+------------------+
                    // |  dirty memory      |  image   |   dirty memory   |
                    // +----------------+---+----------+------------------+
                    //
                    //  <------+-------> <-------------+----------------->
                    //         |                       |
                    //         |                       |
                    //   memset (1)                 madvise (2)
                    //
                    // Here only a single memset is necessary since the image
                    // started after the threshold which we're keeping resident.
                    // Note that the memset may be zero bytes here.

                    // This is memset (1)
                    std::ptr::write_bytes(self.base.as_ptr(), 0u8, keep_resident);

                    // This is madvise (2)
                    self.madvise_reset(keep_resident, self.accessible - keep_resident)?;
                }
            }

            // If there's no memory image for this slot then memset the first
            // bytes in the memory back to zero while using `madvise` to purge
            // the rest.
            None => {
                let size_to_memset = keep_resident.min(self.accessible);
                std::ptr::write_bytes(self.base.as_ptr(), 0u8, size_to_memset);
                self.madvise_reset(size_to_memset, self.accessible - size_to_memset)?;
            }
        }

        Ok(())
    }

    #[allow(dead_code)] // ignore warnings as this is only used in some cfgs
    unsafe fn madvise_reset(&self, base: usize, len: usize) -> Result<()> {
        assert!(base + len <= self.accessible);
        if len == 0 {
            return Ok(());
        }
        vm::madvise_dontneed(self.base.as_ptr().add(base), len)?;
        Ok(())
    }

    fn set_protection(&self, range: Range<usize>, readwrite: bool) -> Result<()> {
        assert!(range.start <= range.end);
        assert!(range.end <= self.static_size);
        if range.len() == 0 {
            return Ok(());
        }

        unsafe {
            let start = self.base.as_ptr().add(range.start);
            if readwrite {
                vm::expose_existing_mapping(start, range.len())?;
            } else {
                vm::hide_existing_mapping(start, range.len())?;
            }
        }

        Ok(())
    }

    pub(crate) fn has_image(&self) -> bool {
        self.image.is_some()
    }

    #[allow(dead_code)] // ignore warnings as this is only used in some cfgs
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Map anonymous zeroed memory across the whole slot,
    /// inaccessible. Used both during instantiate and during drop.
    fn reset_with_anon_memory(&mut self) -> Result<()> {
        if self.static_size == 0 {
            assert!(self.image.is_none());
            assert_eq!(self.accessible, 0);
            return Ok(());
        }

        unsafe {
            vm::erase_existing_mapping(self.base.as_ptr(), self.static_size)?;
        }

        self.image = None;
        self.accessible = 0;

        Ok(())
    }
}

impl Drop for MemoryImageSlot {
    fn drop(&mut self) {
        // The MemoryImageSlot may be dropped if there is an error during
        // instantiation: for example, if a memory-growth limiter
        // disallows a guest from having a memory of a certain size,
        // after we've already initialized the MemoryImageSlot.
        //
        // We need to return this region of the large pool mmap to a
        // safe state (with no module-specific mappings). The
        // MemoryImageSlot will not be returned to the MemoryPool, so a new
        // MemoryImageSlot will be created and overwrite the mappings anyway
        // on the slot's next use; but for safety and to avoid
        // resource leaks it's better not to have stale mappings to a
        // possibly-otherwise-dead module's image.
        //
        // To "wipe the slate clean", let's do a mmap of anonymous
        // memory over the whole region, with PROT_NONE. Note that we
        // *can't* simply munmap, because that leaves a hole in the
        // middle of the pooling allocator's big memory area that some
        // other random mmap may swoop in and take, to be trampled
        // over by the next MemoryImageSlot later.
        //
        // Since we're in drop(), we can't sanely return an error if
        // this mmap fails. Instead though the result is unwrapped here to
        // trigger a panic if something goes wrong. Otherwise if this
        // reset-the-mapping fails then on reuse it might be possible, depending
        // on precisely where errors happened, that stale memory could get
        // leaked through.
        //
        // The exception to all of this is if the `clear_on_drop` flag
        // (which is set by default) is false. If so, the owner of
        // this MemoryImageSlot has indicated that it will clean up in some
        // other way.
        if self.clear_on_drop {
            self.reset_with_anon_memory().unwrap();
        }
    }
}

#[cfg(all(test, target_os = "linux", not(miri)))]
mod test {
    use std::sync::Arc;

    use super::{MemoryImage, MemoryImageSlot, MemoryImageSource, MemoryPlan, MemoryStyle};
    use crate::mmap::Mmap;
    use anyhow::Result;
    use wasmtime_environ::Memory;

    fn create_memfd_with_data(offset: usize, data: &[u8]) -> Result<MemoryImage> {
        // Offset must be page-aligned.
        let page_size = crate::page_size();
        assert_eq!(offset & (page_size - 1), 0);

        // The image length is rounded up to the nearest page size
        let image_len = (data.len() + page_size - 1) & !(page_size - 1);

        Ok(MemoryImage {
            source: MemoryImageSource::from_data(data)?.unwrap(),
            len: image_len,
            source_offset: 0,
            linear_memory_offset: offset,
        })
    }

    fn dummy_memory_plan(style: MemoryStyle) -> MemoryPlan {
        MemoryPlan {
            style,
            memory: Memory {
                minimum: 0,
                maximum: None,
                shared: false,
                memory64: false,
            },
            pre_guard_size: 0,
            offset_guard_size: 0,
        }
    }

    #[test]
    fn instantiate_no_image() {
        let plan = dummy_memory_plan(MemoryStyle::Static { bound: 4 << 30 });
        // 4 MiB mmap'd area, not accessible
        let mut mmap = Mmap::accessible_reserved(0, 4 << 20).unwrap();
        // Create a MemoryImageSlot on top of it
        let mut memfd = MemoryImageSlot::create(mmap.as_mut_ptr() as *mut _, 0, 4 << 20);
        memfd.no_clear_on_drop();
        assert!(!memfd.is_dirty());
        // instantiate with 64 KiB initial size
        memfd.instantiate(64 << 10, None, &plan).unwrap();
        assert!(memfd.is_dirty());
        // We should be able to access this 64 KiB (try both ends) and
        // it should consist of zeroes.
        let slice = unsafe { mmap.slice_mut(0..65536) };
        assert_eq!(0, slice[0]);
        assert_eq!(0, slice[65535]);
        slice[1024] = 42;
        assert_eq!(42, slice[1024]);
        // grow the heap
        memfd.set_heap_limit(128 << 10).unwrap();
        let slice = unsafe { mmap.slice(0..1 << 20) };
        assert_eq!(42, slice[1024]);
        assert_eq!(0, slice[131071]);
        // instantiate again; we should see zeroes, even as the
        // reuse-anon-mmap-opt kicks in
        memfd.clear_and_remain_ready(0).unwrap();
        assert!(!memfd.is_dirty());
        memfd.instantiate(64 << 10, None, &plan).unwrap();
        let slice = unsafe { mmap.slice(0..65536) };
        assert_eq!(0, slice[1024]);
    }

    #[test]
    fn instantiate_image() {
        let plan = dummy_memory_plan(MemoryStyle::Static { bound: 4 << 30 });
        // 4 MiB mmap'd area, not accessible
        let mut mmap = Mmap::accessible_reserved(0, 4 << 20).unwrap();
        // Create a MemoryImageSlot on top of it
        let mut memfd = MemoryImageSlot::create(mmap.as_mut_ptr() as *mut _, 0, 4 << 20);
        memfd.no_clear_on_drop();
        // Create an image with some data.
        let image = Arc::new(create_memfd_with_data(4096, &[1, 2, 3, 4]).unwrap());
        // Instantiate with this image
        memfd.instantiate(64 << 10, Some(&image), &plan).unwrap();
        assert!(memfd.has_image());
        let slice = unsafe { mmap.slice_mut(0..65536) };
        assert_eq!(&[1, 2, 3, 4], &slice[4096..4100]);
        slice[4096] = 5;
        // Clear and re-instantiate same image
        memfd.clear_and_remain_ready(0).unwrap();
        memfd.instantiate(64 << 10, Some(&image), &plan).unwrap();
        let slice = unsafe { mmap.slice_mut(0..65536) };
        // Should not see mutation from above
        assert_eq!(&[1, 2, 3, 4], &slice[4096..4100]);
        // Clear and re-instantiate no image
        memfd.clear_and_remain_ready(0).unwrap();
        memfd.instantiate(64 << 10, None, &plan).unwrap();
        assert!(!memfd.has_image());
        let slice = unsafe { mmap.slice_mut(0..65536) };
        assert_eq!(&[0, 0, 0, 0], &slice[4096..4100]);
        // Clear and re-instantiate image again
        memfd.clear_and_remain_ready(0).unwrap();
        memfd.instantiate(64 << 10, Some(&image), &plan).unwrap();
        let slice = unsafe { mmap.slice_mut(0..65536) };
        assert_eq!(&[1, 2, 3, 4], &slice[4096..4100]);
        // Create another image with different data.
        let image2 = Arc::new(create_memfd_with_data(4096, &[10, 11, 12, 13]).unwrap());
        memfd.clear_and_remain_ready(0).unwrap();
        memfd.instantiate(128 << 10, Some(&image2), &plan).unwrap();
        let slice = unsafe { mmap.slice_mut(0..65536) };
        assert_eq!(&[10, 11, 12, 13], &slice[4096..4100]);
        // Instantiate the original image again; we should notice it's
        // a different image and not reuse the mappings.
        memfd.clear_and_remain_ready(0).unwrap();
        memfd.instantiate(64 << 10, Some(&image), &plan).unwrap();
        let slice = unsafe { mmap.slice_mut(0..65536) };
        assert_eq!(&[1, 2, 3, 4], &slice[4096..4100]);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn memset_instead_of_madvise() {
        let plan = dummy_memory_plan(MemoryStyle::Static { bound: 100 });
        let mut mmap = Mmap::accessible_reserved(0, 4 << 20).unwrap();
        let mut memfd = MemoryImageSlot::create(mmap.as_mut_ptr() as *mut _, 0, 4 << 20);
        memfd.no_clear_on_drop();

        // Test basics with the image
        for image_off in [0, 4096, 8 << 10] {
            let image = Arc::new(create_memfd_with_data(image_off, &[1, 2, 3, 4]).unwrap());
            for amt_to_memset in [0, 4096, 10 << 12, 1 << 20, 10 << 20] {
                memfd.instantiate(64 << 10, Some(&image), &plan).unwrap();
                assert!(memfd.has_image());
                let slice = unsafe { mmap.slice_mut(0..64 << 10) };
                if image_off > 0 {
                    assert_eq!(slice[image_off - 1], 0);
                }
                assert_eq!(slice[image_off + 5], 0);
                assert_eq!(&[1, 2, 3, 4], &slice[image_off..][..4]);
                slice[image_off] = 5;
                assert_eq!(&[5, 2, 3, 4], &slice[image_off..][..4]);
                memfd.clear_and_remain_ready(amt_to_memset).unwrap();
            }
        }

        // Test without an image
        for amt_to_memset in [0, 4096, 10 << 12, 1 << 20, 10 << 20] {
            memfd.instantiate(64 << 10, None, &plan).unwrap();
            let mem = unsafe { mmap.slice_mut(0..64 << 10) };
            for chunk in mem.chunks_mut(1024) {
                assert_eq!(chunk[0], 0);
                chunk[0] = 5;
            }
            memfd.clear_and_remain_ready(amt_to_memset).unwrap();
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn dynamic() {
        let plan = dummy_memory_plan(MemoryStyle::Dynamic { reserve: 200 });

        let mut mmap = Mmap::accessible_reserved(0, 4 << 20).unwrap();
        let mut memfd = MemoryImageSlot::create(mmap.as_mut_ptr() as *mut _, 0, 4 << 20);
        memfd.no_clear_on_drop();
        let image = Arc::new(create_memfd_with_data(4096, &[1, 2, 3, 4]).unwrap());
        let initial = 64 << 10;

        // Instantiate the image and test that memory remains accessible after
        // it's cleared.
        memfd.instantiate(initial, Some(&image), &plan).unwrap();
        assert!(memfd.has_image());
        let slice = unsafe { mmap.slice_mut(0..(64 << 10) + 4096) };
        assert_eq!(&[1, 2, 3, 4], &slice[4096..4100]);
        slice[4096] = 5;
        assert_eq!(&[5, 2, 3, 4], &slice[4096..4100]);
        memfd.clear_and_remain_ready(0).unwrap();
        assert_eq!(&[1, 2, 3, 4], &slice[4096..4100]);

        // Re-instantiate make sure it preserves memory. Grow a bit and set data
        // beyond the initial size.
        memfd.instantiate(initial, Some(&image), &plan).unwrap();
        assert_eq!(&[1, 2, 3, 4], &slice[4096..4100]);
        memfd.set_heap_limit(initial * 2).unwrap();
        assert_eq!(&[0, 0], &slice[initial..initial + 2]);
        slice[initial] = 100;
        assert_eq!(&[100, 0], &slice[initial..initial + 2]);
        memfd.clear_and_remain_ready(0).unwrap();

        // Test that memory is still accessible, but it's been reset
        assert_eq!(&[0, 0], &slice[initial..initial + 2]);

        // Instantiate again, and again memory beyond the initial size should
        // still be accessible. Grow into it again and make sure it works.
        memfd.instantiate(initial, Some(&image), &plan).unwrap();
        assert_eq!(&[0, 0], &slice[initial..initial + 2]);
        memfd.set_heap_limit(initial * 2).unwrap();
        assert_eq!(&[0, 0], &slice[initial..initial + 2]);
        slice[initial] = 100;
        assert_eq!(&[100, 0], &slice[initial..initial + 2]);
        memfd.clear_and_remain_ready(0).unwrap();

        // Reset the image to none and double-check everything is back to zero
        memfd.instantiate(64 << 10, None, &plan).unwrap();
        assert!(!memfd.has_image());
        assert_eq!(&[0, 0, 0, 0], &slice[4096..4100]);
        assert_eq!(&[0, 0], &slice[initial..initial + 2]);
    }
}
