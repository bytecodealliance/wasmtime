//! Memory management for linear memories.
//!
//! `RuntimeLinearMemory` is to WebAssembly linear memories what `Table` is to WebAssembly tables.

use crate::prelude::*;
use crate::runtime::vm::mmap::Mmap;
use crate::runtime::vm::vmcontext::VMMemoryDefinition;
use crate::runtime::vm::{
    round_usize_up_to_host_pages, usize_is_multiple_of_host_page_size, MemoryImage,
    MemoryImageSlot, SendSyncPtr, SharedMemory, VMStore, WaitResult,
};
use alloc::sync::Arc;
use core::ops::Range;
use core::ptr::NonNull;
use core::time::Duration;
use wasmtime_environ::{Trap, Tunables};

/// A memory allocator
pub trait RuntimeMemoryCreator: Send + Sync {
    /// Create new RuntimeLinearMemory
    fn new_memory(
        &self,
        ty: &wasmtime_environ::Memory,
        tunables: &Tunables,
        minimum: usize,
        maximum: Option<usize>,
    ) -> Result<Box<dyn RuntimeLinearMemory>>;
}

/// A default memory allocator used by Wasmtime
pub struct DefaultMemoryCreator;

impl RuntimeMemoryCreator for DefaultMemoryCreator {
    /// Create new MmapMemory
    fn new_memory(
        &self,
        ty: &wasmtime_environ::Memory,
        tunables: &Tunables,
        minimum: usize,
        maximum: Option<usize>,
    ) -> Result<Box<dyn RuntimeLinearMemory>> {
        Ok(Box::new(MmapMemory::new(ty, tunables, minimum, maximum)?))
    }
}

/// A linear memory and its backing storage.
pub trait RuntimeLinearMemory: Send + Sync {
    /// Returns the number bytes that this linear memory can access.
    fn byte_size(&self) -> usize;

    /// Returns the maximal number of bytes the current allocation can access.
    ///
    /// Growth up to this value should not relocate the base pointer.
    fn byte_capacity(&self) -> usize;

    /// Grow memory to the specified amount of bytes.
    ///
    /// Returns an error if memory can't be grown by the specified amount
    /// of bytes.
    fn grow_to(&mut self, size: usize) -> Result<()>;

    /// Returns a pointer to the base of this linear memory allocation.
    fn base_ptr(&mut self) -> *mut u8;

    /// Returns the range of addresses that may be reached by WebAssembly.
    ///
    /// This starts at the base of linear memory and ends at the end of the
    /// guard pages, if any.
    fn wasm_accessible(&self) -> Range<usize>;

    /// Internal method for Wasmtime when used in conjunction with CoW images.
    /// This is used to inform the underlying memory that the size of memory has
    /// changed.
    ///
    /// Note that this is hidden and panics by default as embedders using custom
    /// memory without CoW images shouldn't have to worry about this.
    #[doc(hidden)]
    fn set_byte_size(&mut self, len: usize) {
        let _ = len;
        panic!("CoW images used with this memory and it doesn't support it");
    }
}

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

    fn base_ptr(&mut self) -> *mut u8 {
        unsafe { self.mmap.as_mut_ptr().add(self.pre_guard_size) }
    }

    fn wasm_accessible(&self) -> Range<usize> {
        let base = self.mmap.as_ptr() as usize + self.pre_guard_size;
        let end = base + (self.mmap.len() - self.pre_guard_size);
        base..end
    }
}

/// A "static" memory where the lifetime of the backing memory is managed
/// elsewhere. Currently used with the pooling allocator.
struct StaticMemory {
    /// The base pointer of this static memory, wrapped up in a send/sync
    /// wrapper.
    base: SendSyncPtr<u8>,

    /// The byte capacity of the `base` pointer.
    capacity: usize,

    /// The current size, in bytes, of this memory.
    size: usize,

    /// The size, in bytes, of the virtual address allocation starting at `base`
    /// and going to the end of the guard pages at the end of the linear memory.
    memory_and_guard_size: usize,
}

impl StaticMemory {
    fn new(
        base_ptr: *mut u8,
        base_capacity: usize,
        initial_size: usize,
        maximum_size: Option<usize>,
        memory_and_guard_size: usize,
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
            memory_and_guard_size,
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

    fn base_ptr(&mut self) -> *mut u8 {
        self.base.as_ptr()
    }

    fn wasm_accessible(&self) -> Range<usize> {
        let base = self.base.as_ptr() as usize;
        let end = base + self.memory_and_guard_size;
        base..end
    }
}

/// Representation of a runtime wasm linear memory.
pub enum Memory {
    Local(LocalMemory),
    Shared(SharedMemory),
}

impl Memory {
    /// Create a new dynamic (movable) memory instance for the specified plan.
    pub fn new_dynamic(
        ty: &wasmtime_environ::Memory,
        tunables: &Tunables,
        creator: &dyn RuntimeMemoryCreator,
        store: &mut dyn VMStore,
        memory_image: Option<&Arc<MemoryImage>>,
    ) -> Result<Self> {
        let (minimum, maximum) = Self::limit_new(ty, Some(store))?;
        let allocation = creator.new_memory(ty, tunables, minimum, maximum)?;
        let memory = LocalMemory::new(ty, tunables, allocation, memory_image)?;
        Ok(if ty.shared {
            Memory::Shared(SharedMemory::wrap(ty, memory)?)
        } else {
            Memory::Local(memory)
        })
    }

    /// Create a new static (immovable) memory instance for the specified plan.
    pub fn new_static(
        ty: &wasmtime_environ::Memory,
        tunables: &Tunables,
        base_ptr: *mut u8,
        base_capacity: usize,
        memory_image: MemoryImageSlot,
        memory_and_guard_size: usize,
        store: &mut dyn VMStore,
    ) -> Result<Self> {
        let (minimum, maximum) = Self::limit_new(ty, Some(store))?;
        let pooled_memory = StaticMemory::new(
            base_ptr,
            base_capacity,
            minimum,
            maximum,
            memory_and_guard_size,
        )?;
        let allocation = Box::new(pooled_memory);

        // Configure some defaults a bit differently for this memory within the
        // `LocalMemory` structure created, notably we already have
        // `memory_image` and regardless of configuration settings this memory
        // can't move its base pointer since it's a fixed allocation.
        let mut memory = LocalMemory::new(ty, tunables, allocation, None)?;
        assert!(memory.memory_image.is_none());
        memory.memory_image = Some(memory_image);
        memory.memory_may_move = false;

        Ok(if ty.shared {
            // FIXME(#4244): not supported with the pooling allocator (which
            // `new_static` is always used with), see `MemoryPool::validate` as
            // well).
            todo!("using shared memory with the pooling allocator is a work in progress");
        } else {
            Memory::Local(memory)
        })
    }

    /// Calls the `store`'s limiter to optionally prevent a memory from being allocated.
    ///
    /// Returns a tuple of the minimum size, optional maximum size, and log(page
    /// size) of the memory, all in bytes.
    pub(crate) fn limit_new(
        ty: &wasmtime_environ::Memory,
        store: Option<&mut dyn VMStore>,
    ) -> Result<(usize, Option<usize>)> {
        let page_size = usize::try_from(ty.page_size()).unwrap();

        // This is the absolute possible maximum that the module can try to
        // allocate, which is our entire address space minus a wasm page. That
        // shouldn't ever actually work in terms of an allocation because
        // presumably the kernel wants *something* for itself, but this is used
        // to pass to the `store`'s limiter for a requested size
        // to approximate the scale of the request that the wasm module is
        // making. This is necessary because the limiter works on `usize` bytes
        // whereas we're working with possibly-overflowing `u64` calculations
        // here. To actually faithfully represent the byte requests of modules
        // we'd have to represent things as `u128`, but that's kinda
        // overkill for this purpose.
        let absolute_max = 0usize.wrapping_sub(page_size);

        // Sanity-check what should already be true from wasm module validation.
        if let Ok(size) = ty.minimum_byte_size() {
            assert!(size <= u64::try_from(absolute_max).unwrap());
        }
        if let Ok(max) = ty.maximum_byte_size() {
            assert!(max <= u64::try_from(absolute_max).unwrap());
        }

        // If the minimum memory size overflows the size of our own address
        // space, then we can't satisfy this request, but defer the error to
        // later so the `store` can be informed that an effective oom is
        // happening.
        let minimum = ty
            .minimum_byte_size()
            .ok()
            .and_then(|m| usize::try_from(m).ok());

        // The plan stores the maximum size in units of wasm pages, but we
        // use units of bytes. Unlike for the `minimum` size we silently clamp
        // the effective maximum size to the limits of what we can track. If the
        // maximum size exceeds `usize` or `u64` then there's no need to further
        // keep track of it as some sort of runtime limit will kick in long
        // before we reach the statically declared maximum size.
        let maximum = ty
            .maximum_byte_size()
            .ok()
            .and_then(|m| usize::try_from(m).ok());

        // Inform the store's limiter what's about to happen. This will let the
        // limiter reject anything if necessary, and this also guarantees that
        // we should call the limiter for all requested memories, even if our
        // `minimum` calculation overflowed. This means that the `minimum` we're
        // informing the limiter is lossy and may not be 100% accurate, but for
        // now the expected uses of limiter means that's ok.
        if let Some(store) = store {
            if !store.memory_growing(0, minimum.unwrap_or(absolute_max), maximum)? {
                bail!(
                    "memory minimum size of {} pages exceeds memory limits",
                    ty.limits.min
                );
            }
        }

        // At this point we need to actually handle overflows, so bail out with
        // an error if we made it this far.
        let minimum = minimum.ok_or_else(|| {
            format_err!(
                "memory minimum size of {} pages exceeds memory limits",
                ty.limits.min
            )
        })?;

        Ok((minimum, maximum))
    }

    /// Returns this memory's page size, in bytes.
    pub fn page_size(&self) -> u64 {
        match self {
            Memory::Local(mem) => mem.page_size(),
            Memory::Shared(mem) => mem.page_size(),
        }
    }

    /// Returns the number of allocated wasm pages.
    pub fn byte_size(&self) -> usize {
        match self {
            Memory::Local(mem) => mem.byte_size(),
            Memory::Shared(mem) => mem.byte_size(),
        }
    }

    /// Returns whether or not this memory needs initialization. It
    /// may not if it already has initial content thanks to a CoW
    /// mechanism.
    pub(crate) fn needs_init(&self) -> bool {
        match self {
            Memory::Local(mem) => mem.needs_init(),
            Memory::Shared(mem) => mem.needs_init(),
        }
    }

    /// Grow memory by the specified amount of wasm pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of wasm pages. Returns `Some` with the old size of memory, in bytes, on
    /// successful growth.
    ///
    /// # Safety
    ///
    /// Resizing the memory can reallocate the memory buffer for dynamic memories.
    /// An instance's `VMContext` may have pointers to the memory's base and will
    /// need to be fixed up after growing the memory.
    ///
    /// Generally, prefer using `InstanceHandle::memory_grow`, which encapsulates
    /// this unsafety.
    ///
    /// Ensure that the provided Store is not used to get access any Memory
    /// which lives inside it.
    pub unsafe fn grow(
        &mut self,
        delta_pages: u64,
        store: Option<&mut dyn VMStore>,
    ) -> Result<Option<usize>, Error> {
        let result = match self {
            Memory::Local(mem) => mem.grow(delta_pages, store)?,
            Memory::Shared(mem) => mem.grow(delta_pages, store)?,
        };
        match result {
            Some((old, _new)) => Ok(Some(old)),
            None => Ok(None),
        }
    }

    /// Return a `VMMemoryDefinition` for exposing the memory to compiled wasm code.
    pub fn vmmemory(&mut self) -> VMMemoryDefinition {
        match self {
            Memory::Local(mem) => mem.vmmemory(),
            // `vmmemory()` is used for writing the `VMMemoryDefinition` of a
            // memory into its `VMContext`; this should never be possible for a
            // shared memory because the only `VMMemoryDefinition` for it should
            // be stored in its own `def` field.
            Memory::Shared(_) => unreachable!(),
        }
    }

    /// Consume the memory, returning its [`MemoryImageSlot`] if any is present.
    /// The image should only be present for a subset of memories created with
    /// [`Memory::new_static()`].
    #[cfg(feature = "pooling-allocator")]
    pub fn unwrap_static_image(self) -> MemoryImageSlot {
        match self {
            Memory::Local(mem) => mem.unwrap_static_image(),
            Memory::Shared(_) => panic!("expected a local memory"),
        }
    }

    /// If the [Memory] is a [SharedMemory], unwrap it and return a clone to
    /// that shared memory.
    pub fn as_shared_memory(&mut self) -> Option<&mut SharedMemory> {
        match self {
            Memory::Local(_) => None,
            Memory::Shared(mem) => Some(mem),
        }
    }

    /// Implementation of `memory.atomic.notify` for all memories.
    pub fn atomic_notify(&mut self, addr: u64, count: u32) -> Result<u32, Trap> {
        match self.as_shared_memory() {
            Some(m) => m.atomic_notify(addr, count),
            None => {
                validate_atomic_addr(&self.vmmemory(), addr, 4, 4)?;
                Ok(0)
            }
        }
    }

    /// Implementation of `memory.atomic.wait32` for all memories.
    pub fn atomic_wait32(
        &mut self,
        addr: u64,
        expected: u32,
        timeout: Option<Duration>,
    ) -> Result<WaitResult, Trap> {
        match self.as_shared_memory() {
            Some(m) => m.atomic_wait32(addr, expected, timeout),
            None => {
                validate_atomic_addr(&self.vmmemory(), addr, 4, 4)?;
                Err(Trap::AtomicWaitNonSharedMemory)
            }
        }
    }

    /// Implementation of `memory.atomic.wait64` for all memories.
    pub fn atomic_wait64(
        &mut self,
        addr: u64,
        expected: u64,
        timeout: Option<Duration>,
    ) -> Result<WaitResult, Trap> {
        match self.as_shared_memory() {
            Some(m) => m.atomic_wait64(addr, expected, timeout),
            None => {
                validate_atomic_addr(&self.vmmemory(), addr, 8, 8)?;
                Err(Trap::AtomicWaitNonSharedMemory)
            }
        }
    }

    /// Returns the range of bytes that WebAssembly should be able to address in
    /// this linear memory. Note that this includes guard pages which wasm can
    /// hit.
    pub fn wasm_accessible(&self) -> Range<usize> {
        match self {
            Memory::Local(mem) => mem.wasm_accessible(),
            Memory::Shared(mem) => mem.wasm_accessible(),
        }
    }
}

/// An owned allocation of a wasm linear memory.
///
/// This might be part of a `Memory` via `Memory::Local` but it might also be
/// the implementation basis for a `SharedMemory` behind an `RwLock` for
/// example.
pub struct LocalMemory {
    alloc: Box<dyn RuntimeLinearMemory>,
    ty: wasmtime_environ::Memory,
    memory_may_move: bool,

    /// An optional CoW mapping that provides the initial content of this
    /// memory.
    memory_image: Option<MemoryImageSlot>,
}

impl LocalMemory {
    pub fn new(
        ty: &wasmtime_environ::Memory,
        tunables: &Tunables,
        mut alloc: Box<dyn RuntimeLinearMemory>,
        memory_image: Option<&Arc<MemoryImage>>,
    ) -> Result<LocalMemory> {
        // If a memory image was specified, try to create the MemoryImageSlot on
        // top of our mmap.
        let memory_image = match memory_image {
            Some(image) => {
                let mut slot = MemoryImageSlot::create(
                    alloc.base_ptr().cast(),
                    alloc.byte_size(),
                    alloc.byte_capacity(),
                );
                // On drop, we will unmap our mmap'd range that this slot was
                // mapped on top of, so there is no need for the slot to wipe
                // it with an anonymous mapping first.
                slot.no_clear_on_drop();
                slot.instantiate(alloc.byte_size(), Some(image), ty, tunables)?;
                Some(slot)
            }
            None => None,
        };
        Ok(LocalMemory {
            ty: *ty,
            alloc,
            memory_may_move: ty.memory_may_move(tunables),
            memory_image,
        })
    }

    pub fn page_size(&self) -> u64 {
        self.ty.page_size()
    }

    /// Grows a memory by `delta_pages`.
    ///
    /// This performs the necessary checks on the growth before delegating to
    /// the underlying `grow_to` implementation.
    ///
    /// The `store` is used only for error reporting.
    pub fn grow(
        &mut self,
        delta_pages: u64,
        mut store: Option<&mut dyn VMStore>,
    ) -> Result<Option<(usize, usize)>, Error> {
        let old_byte_size = self.alloc.byte_size();

        // Wasm spec: when growing by 0 pages, always return the current size.
        if delta_pages == 0 {
            return Ok(Some((old_byte_size, old_byte_size)));
        }

        let page_size = usize::try_from(self.page_size()).unwrap();

        // The largest wasm-page-aligned region of memory is possible to
        // represent in a `usize`. This will be impossible for the system to
        // actually allocate.
        let absolute_max = 0usize.wrapping_sub(page_size);

        // Calculate the byte size of the new allocation. Let it overflow up to
        // `usize::MAX`, then clamp it down to `absolute_max`.
        let new_byte_size = usize::try_from(delta_pages)
            .unwrap_or(usize::MAX)
            .saturating_mul(page_size)
            .saturating_add(old_byte_size)
            .min(absolute_max);

        let maximum = self
            .ty
            .maximum_byte_size()
            .ok()
            .and_then(|n| usize::try_from(n).ok());

        // Store limiter gets first chance to reject memory_growing.
        if let Some(store) = &mut store {
            if !store.memory_growing(old_byte_size, new_byte_size, maximum)? {
                return Ok(None);
            }
        }

        // Save the original base pointer to assert the invariant that growth up
        // to the byte capacity never relocates the base pointer.
        let base_ptr_before = self.alloc.base_ptr();
        let required_to_not_move_memory = new_byte_size <= self.alloc.byte_capacity();

        let result = (|| -> Result<()> {
            // Never exceed maximum, even if limiter permitted it.
            if let Some(max) = maximum {
                if new_byte_size > max {
                    bail!("Memory maximum size exceeded");
                }
            }

            // If memory isn't allowed to move then don't let growth happen
            // beyond the initial capacity
            if !self.memory_may_move && new_byte_size > self.alloc.byte_capacity() {
                bail!("Memory maximum size exceeded");
            }

            // If we have a CoW image overlay then let it manage accessible
            // bytes. Once the heap limit is modified inform the underlying
            // allocation that the size has changed.
            //
            // If the growth is going beyond the size of the heap image then
            // discard it. This should only happen for `MmapMemory` where
            // `no_clear_on_drop` is set so the destructor doesn't do anything.
            // For now be maximally sure about this by asserting that memory can
            // indeed move and that we're on unix. If this wants to run
            // somewhere else like Windows or with other allocations this may
            // need adjusting.
            if let Some(image) = &mut self.memory_image {
                if new_byte_size <= self.alloc.byte_capacity() {
                    image.set_heap_limit(new_byte_size)?;
                    self.alloc.set_byte_size(new_byte_size);
                    return Ok(());
                }
                assert!(cfg!(unix));
                assert!(self.memory_may_move);
                self.memory_image = None;
            }

            // And failing all that fall back to the underlying allocation to
            // grow it.
            self.alloc.grow_to(new_byte_size)
        })();

        match result {
            Ok(()) => {
                // On successful growth double-check that the base pointer
                // didn't move if it shouldn't have.
                if required_to_not_move_memory {
                    assert_eq!(base_ptr_before, self.alloc.base_ptr());
                }

                Ok(Some((old_byte_size, new_byte_size)))
            }
            Err(e) => {
                // FIXME: shared memories may not have an associated store to
                // report the growth failure to but the error should not be
                // dropped
                // (https://github.com/bytecodealliance/wasmtime/issues/4240).
                if let Some(store) = store {
                    store.memory_grow_failed(e)?;
                }
                Ok(None)
            }
        }
    }

    pub fn vmmemory(&mut self) -> VMMemoryDefinition {
        VMMemoryDefinition {
            base: self.alloc.base_ptr(),
            current_length: self.alloc.byte_size().into(),
        }
    }

    pub fn byte_size(&self) -> usize {
        self.alloc.byte_size()
    }

    pub fn needs_init(&self) -> bool {
        match &self.memory_image {
            Some(image) => !image.has_image(),
            None => true,
        }
    }

    pub fn wasm_accessible(&self) -> Range<usize> {
        self.alloc.wasm_accessible()
    }

    pub fn unwrap_static_image(self) -> MemoryImageSlot {
        self.memory_image.unwrap()
    }
}

/// In the configurations where bounds checks were elided in JIT code (because
/// we are using static memories with virtual memory guard pages) this manual
/// check is here so we don't segfault from Rust. For other configurations,
/// these checks are required anyways.
pub fn validate_atomic_addr(
    def: &VMMemoryDefinition,
    addr: u64,
    access_size: u64,
    access_alignment: u64,
) -> Result<*mut u8, Trap> {
    debug_assert!(access_alignment.is_power_of_two());
    if !(addr % access_alignment == 0) {
        return Err(Trap::HeapMisaligned);
    }

    let length = u64::try_from(def.current_length()).unwrap();
    if !(addr.saturating_add(access_size) < length) {
        return Err(Trap::MemoryOutOfBounds);
    }

    let addr = usize::try_from(addr).unwrap();
    Ok(def.base.wrapping_add(addr))
}
