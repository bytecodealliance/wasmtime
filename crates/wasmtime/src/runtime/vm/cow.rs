//! Copy-on-write initialization support: creation of backing images for
//! modules, and logic to support mapping these backing images into memory.

use super::sys::DecommitBehavior;
use crate::Engine;
use crate::prelude::*;
use crate::runtime::vm::sys::vm::{self, MemoryImageSource, PageMap, reset_with_pagemap};
use crate::runtime::vm::{
    HostAlignedByteCount, MmapOffset, ModuleMemoryImageSource, host_page_size,
};
use alloc::sync::Arc;
use core::fmt;
use core::ops::Range;
use wasmtime_environ::{DefinedMemoryIndex, MemoryInitialization, Module, PrimaryMap, Tunables};

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
    len: HostAlignedByteCount,

    /// Image starts this many bytes into `source`.
    ///
    /// This is 0 for anonymous-backed memfd files and is the offset of the
    /// data section in a `*.cwasm` file for `*.cwasm`-backed images.
    ///
    /// Must be a multiple of the system page size.
    ///
    /// ## Notes
    ///
    /// This currently isn't a `HostAlignedByteCount` because that's a usize and
    /// this, being a file offset, is a u64.
    source_offset: u64,

    /// Image starts this many bytes into heap space.
    ///
    /// Must be a multiple of the system page size.
    linear_memory_offset: HostAlignedByteCount,

    /// The original source of data that this image is derived from.
    module_source: Arc<dyn ModuleMemoryImageSource>,

    /// The offset, within `module_source.wasm_data()`, that this image starts
    /// at.
    module_source_offset: usize,
}

impl MemoryImage {
    fn new(
        engine: &Engine,
        page_size: u32,
        linear_memory_offset: HostAlignedByteCount,
        module_source: &Arc<impl ModuleMemoryImageSource>,
        data_range: Range<usize>,
    ) -> Result<Option<MemoryImage>> {
        let assert_page_aligned = |val: usize| {
            assert_eq!(val % (page_size as usize), 0);
        };
        // Sanity-check that various parameters are page-aligned.
        let len =
            HostAlignedByteCount::new(data_range.len()).expect("memory image data is page-aligned");

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
        let data = &module_source.wasm_data()[data_range.clone()];
        if !engine.config().force_memory_init_memfd {
            if let Some(mmap) = module_source.mmap() {
                let start = mmap.as_ptr() as usize;
                let end = start + mmap.len();
                let data_start = data.as_ptr() as usize;
                let data_end = data_start + data.len();
                assert!(start <= data_start && data_end <= end);
                assert_page_aligned(start);
                assert_page_aligned(data_start);
                assert_page_aligned(data_end);

                #[cfg(feature = "std")]
                if let Some(file) = mmap.original_file() {
                    if let Some(source) = MemoryImageSource::from_file(file) {
                        return Ok(Some(MemoryImage {
                            source,
                            source_offset: u64::try_from(data_start - start).unwrap(),
                            linear_memory_offset,
                            len,
                            module_source: module_source.clone(),
                            module_source_offset: data_range.start,
                        }));
                    }
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
                module_source: module_source.clone(),
                module_source_offset: data_range.start,
            }));
        }

        Ok(None)
    }

    unsafe fn map_at(&self, mmap_base: &MmapOffset) -> Result<()> {
        unsafe {
            mmap_base.map_image_at(
                &self.source,
                self.source_offset,
                self.linear_memory_offset,
                self.len,
            )
        }
    }

    unsafe fn remap_as_zeros_at(&self, base: *mut u8) -> Result<()> {
        unsafe {
            self.source.remap_as_zeros_at(
                base.add(self.linear_memory_offset.byte_count()),
                self.len.byte_count(),
            )?;
        }
        Ok(())
    }
}

impl ModuleMemoryImages {
    /// Create a new `ModuleMemoryImages` for the given module. This can be
    /// passed in as part of a `InstanceAllocationRequest` to speed up
    /// instantiation and execution by using copy-on-write-backed memories.
    pub fn new(
        engine: &Engine,
        module: &Module,
        source: &Arc<impl ModuleMemoryImageSource>,
    ) -> Result<Option<ModuleMemoryImages>> {
        let map = match &module.memory_initialization {
            MemoryInitialization::Static { map } => map,
            _ => return Ok(None),
        };
        let mut memories = PrimaryMap::with_capacity(map.len());
        let page_size = crate::runtime::vm::host_page_size();
        let page_size = u32::try_from(page_size).unwrap();
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

            let data_range = init.data.start as usize..init.data.end as usize;
            if module.memories[memory_index]
                .minimum_byte_size()
                .map_or(false, |mem_initial_len| {
                    init.offset + u64::try_from(data_range.len()).unwrap() > mem_initial_len
                })
            {
                // The image is rounded up to multiples of the host OS page
                // size. But if Wasm is using a custom page size, the Wasm page
                // size might be smaller than the host OS page size, and that
                // rounding might have made the image larger than the Wasm
                // memory's initial length. This is *probably* okay, since the
                // rounding would have just introduced new runs of zeroes in the
                // image, but out of an abundance of caution we don't generate
                // CoW images in this scenario.
                return Ok(None);
            }

            let offset_usize = match usize::try_from(init.offset) {
                Ok(offset) => offset,
                Err(_) => return Ok(None),
            };
            let offset = HostAlignedByteCount::new(offset_usize)
                .expect("memory init offset is a multiple of the host page size");

            // If this creation fails then we fail creating
            // `ModuleMemoryImages` since this memory couldn't be represented.
            let image = match MemoryImage::new(engine, page_size, offset, source, data_range)? {
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
pub struct MemoryImageSlot {
    /// The mmap and offset within it that contains the linear memory for this
    /// slot.
    base: MmapOffset,

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
    ///
    /// Also note that this is always page-aligned.
    accessible: HostAlignedByteCount,

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

impl fmt::Debug for MemoryImageSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryImageSlot")
            .field("base", &self.base)
            .field("static_size", &self.static_size)
            .field("accessible", &self.accessible)
            .field("dirty", &self.dirty)
            .field("clear_on_drop", &self.clear_on_drop)
            .finish_non_exhaustive()
    }
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
    pub(crate) fn create(
        base: MmapOffset,
        accessible: HostAlignedByteCount,
        static_size: usize,
    ) -> Self {
        MemoryImageSlot {
            base,
            static_size,
            accessible,
            image: None,
            dirty: false,
            clear_on_drop: true,
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
        let size_bytes_aligned = HostAlignedByteCount::new_rounded_up(size_bytes)?;
        assert!(size_bytes <= self.static_size);
        assert!(size_bytes_aligned.byte_count() <= self.static_size);

        // If the heap limit already addresses accessible bytes then no syscalls
        // are necessary since the data is already mapped into the process and
        // waiting to go.
        //
        // This is used for "dynamic" memories where memory is not always
        // decommitted during recycling (but it's still always reset).
        if size_bytes_aligned <= self.accessible {
            return Ok(());
        }

        // Otherwise use `mprotect` to make the new pages read/write.
        self.set_protection(self.accessible..size_bytes_aligned, true)?;
        self.accessible = size_bytes_aligned;

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
        ty: &wasmtime_environ::Memory,
        tunables: &Tunables,
    ) -> Result<()> {
        assert!(!self.dirty);
        assert!(
            initial_size_bytes <= self.static_size,
            "initial_size_bytes <= self.static_size failed: \
             initial_size_bytes={initial_size_bytes}, self.static_size={}",
            self.static_size
        );
        let initial_size_bytes_page_aligned =
            HostAlignedByteCount::new_rounded_up(initial_size_bytes)?;

        // First order of business is to blow away the previous linear memory
        // image if it doesn't match the image specified here. If one is
        // detected then it's reset with anonymous memory which means that all
        // of memory up to `self.accessible` will now be read/write and zero.
        //
        // Note that this intentionally a "small mmap" which only covers the
        // extent of the prior initialization image in order to preserve
        // resident memory that might come before or after the image.
        let images_equal = match (self.image.as_ref(), maybe_image) {
            (Some(a), Some(b)) if Arc::ptr_eq(a, b) => true,
            (None, None) => true,
            _ => false,
        };
        if !images_equal {
            self.remove_image()?;
        }

        // The next order of business is to ensure that `self.accessible` is
        // appropriate. First up is to grow the read/write portion of memory if
        // it's not large enough to accommodate `initial_size_bytes`.
        if self.accessible < initial_size_bytes_page_aligned {
            self.set_protection(self.accessible..initial_size_bytes_page_aligned, true)?;
            self.accessible = initial_size_bytes_page_aligned;
        }

        // If (1) the accessible region is not in its initial state, and (2) the
        // memory relies on virtual memory at all (i.e. has offset guard
        // pages), then we need to reset memory protections. Put another way,
        // the only time it is safe to not reset protections is when we are
        // using dynamic memory without any guard pages.
        let host_page_size_log2 = u8::try_from(host_page_size().ilog2()).unwrap();
        if initial_size_bytes_page_aligned < self.accessible
            && (tunables.memory_guard_size > 0
                || ty.can_elide_bounds_check(tunables, host_page_size_log2))
        {
            self.set_protection(initial_size_bytes_page_aligned..self.accessible, false)?;
            self.accessible = initial_size_bytes_page_aligned;
        }

        // Now that memory is sized appropriately the final operation is to
        // place the new image into linear memory. Note that this operation is
        // skipped if `self.image` matches `maybe_image`.
        assert!(initial_size_bytes <= self.accessible.byte_count());
        assert!(initial_size_bytes_page_aligned <= self.accessible);
        if !images_equal {
            if let Some(image) = maybe_image.as_ref() {
                assert!(
                    image
                        .linear_memory_offset
                        .checked_add(image.len)
                        .unwrap()
                        .byte_count()
                        <= initial_size_bytes
                );
                if !image.len.is_zero() {
                    unsafe {
                        image.map_at(&self.base)?;
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
                image.remap_as_zeros_at(self.base.as_mut_ptr())?;
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
    #[allow(dead_code, reason = "only used in some cfgs")]
    pub(crate) fn clear_and_remain_ready(
        &mut self,
        pagemap: Option<&PageMap>,
        keep_resident: HostAlignedByteCount,
        decommit: impl FnMut(*mut u8, usize),
    ) -> Result<()> {
        assert!(self.dirty);

        unsafe {
            self.reset_all_memory_contents(pagemap, keep_resident, decommit)?;
        }

        self.dirty = false;
        Ok(())
    }

    #[allow(dead_code, reason = "only used in some cfgs")]
    unsafe fn reset_all_memory_contents(
        &mut self,
        pagemap: Option<&PageMap>,
        keep_resident: HostAlignedByteCount,
        decommit: impl FnMut(*mut u8, usize),
    ) -> Result<()> {
        match vm::decommit_behavior() {
            DecommitBehavior::Zero => {
                // If we're not on Linux then there's no generic platform way to
                // reset memory back to its original state, so instead reset memory
                // back to entirely zeros with an anonymous backing.
                //
                // Additionally the previous image, if any, is dropped here
                // since it's no longer applicable to this mapping.
                self.reset_with_anon_memory()
            }
            DecommitBehavior::RestoreOriginalMapping => {
                unsafe {
                    self.reset_with_original_mapping(pagemap, keep_resident, decommit);
                }
                Ok(())
            }
        }
    }

    #[allow(dead_code, reason = "only used in some cfgs")]
    unsafe fn reset_with_original_mapping(
        &mut self,
        pagemap: Option<&PageMap>,
        keep_resident: HostAlignedByteCount,
        decommit: impl FnMut(*mut u8, usize),
    ) {
        assert_eq!(
            vm::decommit_behavior(),
            DecommitBehavior::RestoreOriginalMapping
        );

        unsafe {
            match &self.image {
                // If there's a backing image then manually resetting a region
                // is a bit trickier than without an image, so delegate to the
                // helper function below.
                Some(image) => {
                    reset_with_pagemap(
                        pagemap,
                        self.base.as_mut_ptr(),
                        self.accessible,
                        keep_resident,
                        |region| {
                            manually_reset_region(self.base.as_mut_ptr().addr(), image, region)
                        },
                        decommit,
                    );
                }

                // If there's no memory image for this slot then pages are always
                // manually reset back to zero or given to `decommit`.
                None => reset_with_pagemap(
                    pagemap,
                    self.base.as_mut_ptr(),
                    self.accessible,
                    keep_resident,
                    |region| region.fill(0),
                    decommit,
                ),
            }
        }

        /// Manually resets `region` back to its original contents as specified
        /// in `image`.
        ///
        /// This assumes that the original mmap starts at `base_addr` and
        /// `region` is a subslice within the original mmap.
        ///
        /// # Panics
        ///
        /// Panics if `base_addr` is not the right index due to the various
        /// indexing calculations below.
        fn manually_reset_region(base_addr: usize, image: &MemoryImage, mut region: &mut [u8]) {
            let image_start = image.linear_memory_offset.byte_count();
            let image_end = image_start + image.len.byte_count();
            let mut region_start = region.as_ptr().addr() - base_addr;
            let region_end = region_start + region.len();
            let image_bytes = image.module_source.wasm_data();
            let image_bytes = &image_bytes[image.module_source_offset..][..image.len.byte_count()];

            // 1. Zero out the part before the image (if any).
            if let Some(len_before_image) = image_start.checked_sub(region_start) {
                let len = len_before_image.min(region.len());
                let (a, b) = region.split_at_mut(len);
                a.fill(0);
                region = b;
                region_start += len;

                if region.is_empty() {
                    return;
                }
            }

            debug_assert_eq!(region_end - region_start, region.len());
            debug_assert!(region_start >= image_start);

            // 2. Copy the original bytes from the image for the part that
            //    overlaps with the image.
            if let Some(len_in_image) = image_end.checked_sub(region_start) {
                let len = len_in_image.min(region.len());
                let (a, b) = region.split_at_mut(len);
                a.copy_from_slice(&image_bytes[region_start - image_start..][..len]);
                region = b;
                region_start += len;

                if region.is_empty() {
                    return;
                }
            }

            debug_assert_eq!(region_end - region_start, region.len());
            debug_assert!(region_start >= image_end);

            // 3. Zero out the part after the image.
            region.fill(0);
        }
    }

    fn set_protection(&self, range: Range<HostAlignedByteCount>, readwrite: bool) -> Result<()> {
        let len = range
            .end
            .checked_sub(range.start)
            .expect("range.start <= range.end");
        assert!(range.end.byte_count() <= self.static_size);
        if len.is_zero() {
            return Ok(());
        }

        // TODO: use Mmap to change memory permissions instead of these free
        // functions.
        unsafe {
            let start = self.base.as_mut_ptr().add(range.start.byte_count());
            if readwrite {
                vm::expose_existing_mapping(start, len.byte_count())?;
            } else {
                vm::hide_existing_mapping(start, len.byte_count())?;
            }
        }

        Ok(())
    }

    pub(crate) fn has_image(&self) -> bool {
        self.image.is_some()
    }

    #[allow(dead_code, reason = "only used in some cfgs")]
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
            vm::erase_existing_mapping(self.base.as_mut_ptr(), self.static_size)?;
        }

        self.image = None;
        self.accessible = HostAlignedByteCount::ZERO;

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
    use super::*;
    use crate::runtime::vm::mmap::{AlignedLength, Mmap};
    use crate::runtime::vm::sys::vm::decommit_pages;
    use crate::runtime::vm::{HostAlignedByteCount, MmapVec, host_page_size};
    use std::sync::Arc;
    use wasmtime_environ::{IndexType, Limits, Memory};

    fn create_memfd_with_data(offset: usize, data: &[u8]) -> Result<MemoryImage> {
        // offset must be a multiple of the page size.
        let linear_memory_offset =
            HostAlignedByteCount::new(offset).expect("offset is page-aligned");
        // The image length is rounded up to the nearest page size
        let image_len = HostAlignedByteCount::new_rounded_up(data.len()).unwrap();

        let mut source = TestDataSource {
            data: vec![0; image_len.byte_count()],
        };
        source.data[..data.len()].copy_from_slice(data);

        return Ok(MemoryImage {
            source: MemoryImageSource::from_data(data)?.unwrap(),
            len: image_len,
            source_offset: 0,
            linear_memory_offset,
            module_source: Arc::new(source),
            module_source_offset: 0,
        });

        struct TestDataSource {
            data: Vec<u8>,
        }

        impl ModuleMemoryImageSource for TestDataSource {
            fn wasm_data(&self) -> &[u8] {
                &self.data
            }
            fn mmap(&self) -> Option<&MmapVec> {
                None
            }
        }
    }

    fn dummy_memory() -> Memory {
        Memory {
            idx_type: IndexType::I32,
            limits: Limits { min: 0, max: None },
            shared: false,
            page_size_log2: Memory::DEFAULT_PAGE_SIZE_LOG2,
        }
    }

    fn mmap_4mib_inaccessible() -> Arc<Mmap<AlignedLength>> {
        let four_mib = HostAlignedByteCount::new(4 << 20).expect("4 MiB is page aligned");
        Arc::new(Mmap::accessible_reserved(HostAlignedByteCount::ZERO, four_mib).unwrap())
    }

    /// Presents a part of an mmap as a mutable slice within a callback.
    ///
    /// The callback ensures that the reference no longer lives after the
    /// function is done.
    ///
    /// # Safety
    ///
    /// The caller must ensure that during this function call, the only way this
    /// region of memory is not accessed by (read from or written to) is via the
    /// reference. Making the callback `'static` goes some way towards ensuring
    /// that, but it's still possible to squirrel away a reference into global
    /// state. So don't do that.
    unsafe fn with_slice_mut(
        mmap: &Arc<Mmap<AlignedLength>>,
        range: Range<usize>,
        f: impl FnOnce(&mut [u8]) + 'static,
    ) {
        let ptr = mmap.as_ptr().cast_mut();
        let slice = unsafe {
            core::slice::from_raw_parts_mut(ptr.add(range.start), range.end - range.start)
        };
        f(slice);
    }

    #[test]
    fn instantiate_no_image() {
        let ty = dummy_memory();
        let tunables = Tunables {
            memory_reservation: 4 << 30,
            ..Tunables::default_miri()
        };
        // 4 MiB mmap'd area, not accessible
        let mmap = mmap_4mib_inaccessible();
        // Create a MemoryImageSlot on top of it
        let mut memfd =
            MemoryImageSlot::create(mmap.zero_offset(), HostAlignedByteCount::ZERO, 4 << 20);
        memfd.no_clear_on_drop();
        assert!(!memfd.is_dirty());
        // instantiate with 64 KiB initial size
        memfd.instantiate(64 << 10, None, &ty, &tunables).unwrap();
        assert!(memfd.is_dirty());

        // We should be able to access this 64 KiB (try both ends) and
        // it should consist of zeroes.
        unsafe {
            with_slice_mut(&mmap, 0..65536, |slice| {
                assert_eq!(0, slice[0]);
                assert_eq!(0, slice[65535]);
                slice[1024] = 42;
                assert_eq!(42, slice[1024]);
            });
        }

        // grow the heap
        memfd.set_heap_limit(128 << 10).unwrap();
        let slice = unsafe { mmap.slice(0..1 << 20) };
        assert_eq!(42, slice[1024]);
        assert_eq!(0, slice[131071]);
        // instantiate again; we should see zeroes, even as the
        // reuse-anon-mmap-opt kicks in
        memfd
            .clear_and_remain_ready(None, HostAlignedByteCount::ZERO, |ptr, len| unsafe {
                decommit_pages(ptr, len).unwrap()
            })
            .unwrap();
        assert!(!memfd.is_dirty());
        memfd.instantiate(64 << 10, None, &ty, &tunables).unwrap();
        let slice = unsafe { mmap.slice(0..65536) };
        assert_eq!(0, slice[1024]);
    }

    #[test]
    fn instantiate_image() {
        let page_size = host_page_size();
        let ty = dummy_memory();
        let tunables = Tunables {
            memory_reservation: 4 << 30,
            ..Tunables::default_miri()
        };
        // 4 MiB mmap'd area, not accessible
        let mmap = mmap_4mib_inaccessible();
        // Create a MemoryImageSlot on top of it
        let mut memfd =
            MemoryImageSlot::create(mmap.zero_offset(), HostAlignedByteCount::ZERO, 4 << 20);
        memfd.no_clear_on_drop();
        // Create an image with some data.
        let image = Arc::new(create_memfd_with_data(page_size, &[1, 2, 3, 4]).unwrap());
        // Instantiate with this image
        memfd
            .instantiate(64 << 10, Some(&image), &ty, &tunables)
            .unwrap();
        assert!(memfd.has_image());

        unsafe {
            with_slice_mut(&mmap, 0..65536, move |slice| {
                assert_eq!(&[1, 2, 3, 4], &slice[page_size..][..4]);
                slice[page_size] = 5;
            });
        }

        // Clear and re-instantiate same image
        memfd
            .clear_and_remain_ready(None, HostAlignedByteCount::ZERO, |ptr, len| unsafe {
                decommit_pages(ptr, len).unwrap()
            })
            .unwrap();
        memfd
            .instantiate(64 << 10, Some(&image), &ty, &tunables)
            .unwrap();
        let slice = unsafe { mmap.slice(0..65536) };
        assert_eq!(&[1, 2, 3, 4], &slice[page_size..][..4]);

        // Clear and re-instantiate no image
        memfd
            .clear_and_remain_ready(None, HostAlignedByteCount::ZERO, |ptr, len| unsafe {
                decommit_pages(ptr, len).unwrap()
            })
            .unwrap();
        memfd.instantiate(64 << 10, None, &ty, &tunables).unwrap();
        assert!(!memfd.has_image());
        let slice = unsafe { mmap.slice(0..65536) };
        assert_eq!(&[0, 0, 0, 0], &slice[page_size..][..4]);

        // Clear and re-instantiate image again
        memfd
            .clear_and_remain_ready(None, HostAlignedByteCount::ZERO, |ptr, len| unsafe {
                decommit_pages(ptr, len).unwrap()
            })
            .unwrap();
        memfd
            .instantiate(64 << 10, Some(&image), &ty, &tunables)
            .unwrap();
        let slice = unsafe { mmap.slice(0..65536) };
        assert_eq!(&[1, 2, 3, 4], &slice[page_size..][..4]);

        // Create another image with different data.
        let image2 = Arc::new(create_memfd_with_data(page_size, &[10, 11, 12, 13]).unwrap());
        memfd
            .clear_and_remain_ready(None, HostAlignedByteCount::ZERO, |ptr, len| unsafe {
                decommit_pages(ptr, len).unwrap()
            })
            .unwrap();
        memfd
            .instantiate(128 << 10, Some(&image2), &ty, &tunables)
            .unwrap();
        let slice = unsafe { mmap.slice(0..65536) };
        assert_eq!(&[10, 11, 12, 13], &slice[page_size..][..4]);

        // Instantiate the original image again; we should notice it's
        // a different image and not reuse the mappings.
        memfd
            .clear_and_remain_ready(None, HostAlignedByteCount::ZERO, |ptr, len| unsafe {
                decommit_pages(ptr, len).unwrap()
            })
            .unwrap();
        memfd
            .instantiate(64 << 10, Some(&image), &ty, &tunables)
            .unwrap();
        let slice = unsafe { mmap.slice(0..65536) };
        assert_eq!(&[1, 2, 3, 4], &slice[page_size..][..4]);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn memset_instead_of_madvise() {
        let page_size = host_page_size();
        let ty = dummy_memory();
        let tunables = Tunables {
            memory_reservation: 100 << 16,
            ..Tunables::default_miri()
        };
        let mmap = mmap_4mib_inaccessible();
        let mut memfd =
            MemoryImageSlot::create(mmap.zero_offset(), HostAlignedByteCount::ZERO, 4 << 20);
        memfd.no_clear_on_drop();

        // Test basics with the image
        for image_off in [0, page_size, page_size * 2] {
            let image = Arc::new(create_memfd_with_data(image_off, &[1, 2, 3, 4]).unwrap());
            for amt_to_memset in [0, page_size, page_size * 10, 1 << 20, 10 << 20] {
                let amt_to_memset = HostAlignedByteCount::new(amt_to_memset).unwrap();
                memfd
                    .instantiate(64 << 10, Some(&image), &ty, &tunables)
                    .unwrap();
                assert!(memfd.has_image());

                unsafe {
                    with_slice_mut(&mmap, 0..64 << 10, move |slice| {
                        if image_off > 0 {
                            assert_eq!(slice[image_off - 1], 0);
                        }
                        assert_eq!(slice[image_off + 5], 0);
                        assert_eq!(&[1, 2, 3, 4], &slice[image_off..][..4]);
                        slice[image_off] = 5;
                        assert_eq!(&[5, 2, 3, 4], &slice[image_off..][..4]);
                    })
                };

                memfd
                    .clear_and_remain_ready(None, amt_to_memset, |ptr, len| unsafe {
                        decommit_pages(ptr, len).unwrap()
                    })
                    .unwrap();
            }
        }

        // Test without an image
        for amt_to_memset in [0, page_size, page_size * 10, 1 << 20, 10 << 20] {
            let amt_to_memset = HostAlignedByteCount::new(amt_to_memset).unwrap();
            memfd.instantiate(64 << 10, None, &ty, &tunables).unwrap();

            unsafe {
                with_slice_mut(&mmap, 0..64 << 10, |slice| {
                    for chunk in slice.chunks_mut(1024) {
                        assert_eq!(chunk[0], 0);
                        chunk[0] = 5;
                    }
                });
            }
            memfd
                .clear_and_remain_ready(None, amt_to_memset, |ptr, len| unsafe {
                    decommit_pages(ptr, len).unwrap()
                })
                .unwrap();
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn dynamic() {
        let page_size = host_page_size();
        let ty = dummy_memory();
        let tunables = Tunables {
            memory_reservation: 0,
            memory_reservation_for_growth: 200,
            ..Tunables::default_miri()
        };

        let mmap = mmap_4mib_inaccessible();
        let mut memfd =
            MemoryImageSlot::create(mmap.zero_offset(), HostAlignedByteCount::ZERO, 4 << 20);
        memfd.no_clear_on_drop();
        let image = Arc::new(create_memfd_with_data(page_size, &[1, 2, 3, 4]).unwrap());
        let initial = 64 << 10;

        // Instantiate the image and test that memory remains accessible after
        // it's cleared.
        memfd
            .instantiate(initial, Some(&image), &ty, &tunables)
            .unwrap();
        assert!(memfd.has_image());

        unsafe {
            with_slice_mut(&mmap, 0..(64 << 10) + page_size, move |slice| {
                assert_eq!(&[1, 2, 3, 4], &slice[page_size..][..4]);
                slice[page_size] = 5;
                assert_eq!(&[5, 2, 3, 4], &slice[page_size..][..4]);
            });
        }

        memfd
            .clear_and_remain_ready(None, HostAlignedByteCount::ZERO, |ptr, len| unsafe {
                decommit_pages(ptr, len).unwrap()
            })
            .unwrap();
        let slice = unsafe { mmap.slice(0..(64 << 10) + page_size) };
        assert_eq!(&[1, 2, 3, 4], &slice[page_size..][..4]);

        // Re-instantiate make sure it preserves memory. Grow a bit and set data
        // beyond the initial size.
        memfd
            .instantiate(initial, Some(&image), &ty, &tunables)
            .unwrap();
        assert_eq!(&[1, 2, 3, 4], &slice[page_size..][..4]);

        memfd.set_heap_limit(initial * 2).unwrap();

        unsafe {
            with_slice_mut(&mmap, 0..(64 << 10) + page_size, move |slice| {
                assert_eq!(&[0, 0], &slice[initial..initial + 2]);
                slice[initial] = 100;
                assert_eq!(&[100, 0], &slice[initial..initial + 2]);
            });
        }

        memfd
            .clear_and_remain_ready(None, HostAlignedByteCount::ZERO, |ptr, len| unsafe {
                decommit_pages(ptr, len).unwrap()
            })
            .unwrap();

        // Test that memory is still accessible, but it's been reset
        assert_eq!(&[0, 0], &slice[initial..initial + 2]);

        // Instantiate again, and again memory beyond the initial size should
        // still be accessible. Grow into it again and make sure it works.
        memfd
            .instantiate(initial, Some(&image), &ty, &tunables)
            .unwrap();
        assert_eq!(&[0, 0], &slice[initial..initial + 2]);
        memfd.set_heap_limit(initial * 2).unwrap();

        unsafe {
            with_slice_mut(&mmap, 0..(64 << 10) + page_size, move |slice| {
                assert_eq!(&[0, 0], &slice[initial..initial + 2]);
                slice[initial] = 100;
                assert_eq!(&[100, 0], &slice[initial..initial + 2]);
            });
        }

        memfd
            .clear_and_remain_ready(None, HostAlignedByteCount::ZERO, |ptr, len| unsafe {
                decommit_pages(ptr, len).unwrap()
            })
            .unwrap();

        // Reset the image to none and double-check everything is back to zero
        memfd.instantiate(64 << 10, None, &ty, &tunables).unwrap();
        assert!(!memfd.has_image());
        assert_eq!(&[0, 0, 0, 0], &slice[page_size..][..4]);
        assert_eq!(&[0, 0], &slice[initial..initial + 2]);
    }

    #[test]
    fn reset_with_pagemap() {
        let page_size = host_page_size();
        let ty = dummy_memory();
        let tunables = Tunables {
            memory_reservation: 100 << 16,
            ..Tunables::default_miri()
        };
        let mmap = mmap_4mib_inaccessible();
        let mmap_len = page_size * 9;
        let mut memfd =
            MemoryImageSlot::create(mmap.zero_offset(), HostAlignedByteCount::ZERO, mmap_len);
        memfd.no_clear_on_drop();
        let pagemap = PageMap::new();
        let pagemap = pagemap.as_ref();

        let mut data = vec![0; 3 * page_size];
        for (i, chunk) in data.chunks_mut(page_size).enumerate() {
            for slot in chunk {
                *slot = u8::try_from(i + 1).unwrap();
            }
        }
        let image = Arc::new(create_memfd_with_data(3 * page_size, &data).unwrap());

        memfd
            .instantiate(mmap_len, Some(&image), &ty, &tunables)
            .unwrap();

        let keep_resident = HostAlignedByteCount::new(mmap_len).unwrap();
        let assert_pristine_after_reset = |memfd: &mut MemoryImageSlot| unsafe {
            // Wipe the image, keeping some bytes resident.
            memfd
                .clear_and_remain_ready(pagemap, keep_resident, |ptr, len| {
                    decommit_pages(ptr, len).unwrap()
                })
                .unwrap();

            // Double check that the contents of memory are as expected after
            // reset.
            with_slice_mut(&mmap, 0..mmap_len, move |slice| {
                for (i, chunk) in slice.chunks(page_size).enumerate() {
                    let expected = match i {
                        0..3 => 0,
                        3..6 => u8::try_from(i).unwrap() - 2,
                        6..9 => 0,
                        _ => unreachable!(),
                    };
                    for slot in chunk {
                        assert_eq!(*slot, expected);
                    }
                }
            });

            // Re-instantiate, but then wipe the image entirely by keeping
            // nothing resident.
            memfd
                .instantiate(mmap_len, Some(&image), &ty, &tunables)
                .unwrap();
            memfd
                .clear_and_remain_ready(pagemap, HostAlignedByteCount::ZERO, |ptr, len| {
                    decommit_pages(ptr, len).unwrap()
                })
                .unwrap();

            // Next re-instantiate a final time to get used for the next test.
            memfd
                .instantiate(mmap_len, Some(&image), &ty, &tunables)
                .unwrap();
        };

        let write_page = |_memfd: &mut MemoryImageSlot, page: usize| unsafe {
            with_slice_mut(
                &mmap,
                page * page_size..(page + 1) * page_size,
                move |slice| slice.fill(0xff),
            );
        };

        // Test various combinations of dirty pages and regions. For example
        // test a dirty region of memory entirely in the zero-initialized zone
        // before/after the image and also test when the dirty region straddles
        // just the start of the image, just the end of the image, both ends,
        // and is entirely contained in just the image.
        assert_pristine_after_reset(&mut memfd);

        for i in 0..9 {
            write_page(&mut memfd, i);
            assert_pristine_after_reset(&mut memfd);
        }
        write_page(&mut memfd, 0);
        write_page(&mut memfd, 1);
        assert_pristine_after_reset(&mut memfd);
        write_page(&mut memfd, 1);
        assert_pristine_after_reset(&mut memfd);
        write_page(&mut memfd, 2);
        write_page(&mut memfd, 3);
        assert_pristine_after_reset(&mut memfd);
        write_page(&mut memfd, 3);
        write_page(&mut memfd, 4);
        write_page(&mut memfd, 5);
        assert_pristine_after_reset(&mut memfd);
        write_page(&mut memfd, 0);
        write_page(&mut memfd, 1);
        write_page(&mut memfd, 2);
        assert_pristine_after_reset(&mut memfd);
        write_page(&mut memfd, 0);
        write_page(&mut memfd, 3);
        write_page(&mut memfd, 6);
        assert_pristine_after_reset(&mut memfd);
        write_page(&mut memfd, 2);
        write_page(&mut memfd, 3);
        write_page(&mut memfd, 4);
        write_page(&mut memfd, 5);
        write_page(&mut memfd, 6);
        assert_pristine_after_reset(&mut memfd);
        write_page(&mut memfd, 4);
        write_page(&mut memfd, 5);
        write_page(&mut memfd, 6);
        write_page(&mut memfd, 7);
        assert_pristine_after_reset(&mut memfd);
        write_page(&mut memfd, 4);
        write_page(&mut memfd, 5);
        write_page(&mut memfd, 8);
        assert_pristine_after_reset(&mut memfd);
    }
}
