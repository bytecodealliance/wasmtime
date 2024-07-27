use crate::prelude::*;
use crate::runtime::vm::{RuntimeLinearMemory, VMMemoryImport};
use crate::store::{StoreData, StoreOpaque, Stored};
use crate::trampoline::generate_memory_export;
use crate::Trap;
use crate::{AsContext, AsContextMut, Engine, MemoryType, StoreContext, StoreContextMut};
use core::cell::UnsafeCell;
use core::fmt;
use core::ops::Range;
use core::slice;
use core::time::Duration;
use wasmtime_environ::MemoryPlan;

pub use crate::runtime::vm::WaitResult;

/// Error for out of bounds [`Memory`] access.
#[derive(Debug)]
#[non_exhaustive]
pub struct MemoryAccessError {
    // Keep struct internals private for future extensibility.
    _private: (),
}

impl fmt::Display for MemoryAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "out of bounds memory access")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MemoryAccessError {}

/// A WebAssembly linear memory.
///
/// WebAssembly memories represent a contiguous array of bytes that have a size
/// that is always a multiple of the WebAssembly page size, currently 64
/// kilobytes.
///
/// WebAssembly memory is used for global data (not to be confused with wasm
/// `global` items), statics in C/C++/Rust, shadow stack memory, etc. Accessing
/// wasm memory is generally quite fast.
///
/// Memories, like other wasm items, are owned by a [`Store`](crate::Store).
///
/// # `Memory` and Safety
///
/// Linear memory is a lynchpin of safety for WebAssembly. In Wasmtime there are
/// safe methods of interacting with a [`Memory`]:
///
/// * [`Memory::read`]
/// * [`Memory::write`]
/// * [`Memory::data`]
/// * [`Memory::data_mut`]
///
/// Note that all of these consider the entire store context as borrowed for the
/// duration of the call or the duration of the returned slice. This largely
/// means that while the function is running you'll be unable to borrow anything
/// else from the store. This includes getting access to the `T` on
/// [`Store<T>`](crate::Store), but it also means that you can't recursively
/// call into WebAssembly for instance.
///
/// If you'd like to dip your toes into handling [`Memory`] in a more raw
/// fashion (e.g. by using raw pointers or raw slices), then there's a few
/// important points to consider when doing so:
///
/// * Any recursive calls into WebAssembly can possibly modify any byte of the
///   entire memory. This means that whenever wasm is called Rust can't have any
///   long-lived borrows live across the wasm function call. Slices like `&mut
///   [u8]` will be violated because they're not actually exclusive at that
///   point, and slices like `&[u8]` are also violated because their contents
///   may be mutated.
///
/// * WebAssembly memories can grow, and growth may change the base pointer.
///   This means that even holding a raw pointer to memory over a wasm function
///   call is also incorrect. Anywhere in the function call the base address of
///   memory may change. Note that growth can also be requested from the
///   embedding API as well.
///
/// As a general rule of thumb it's recommended to stick to the safe methods of
/// [`Memory`] if you can. It's not advised to use raw pointers or `unsafe`
/// operations because of how easy it is to accidentally get things wrong.
///
/// Some examples of safely interacting with memory are:
///
/// ```rust
/// use wasmtime::{Memory, Store, MemoryAccessError};
///
/// // Memory can be read and written safely with the `Memory::read` and
/// // `Memory::write` methods.
/// // An error is returned if the copy did not succeed.
/// fn safe_examples(mem: Memory, store: &mut Store<()>) -> Result<(), MemoryAccessError> {
///     let offset = 5;
///     mem.write(&mut *store, offset, b"hello")?;
///     let mut buffer = [0u8; 5];
///     mem.read(&store, offset, &mut buffer)?;
///     assert_eq!(b"hello", &buffer);
///
///     // Note that while this is safe care must be taken because the indexing
///     // here may panic if the memory isn't large enough.
///     assert_eq!(&mem.data(&store)[offset..offset + 5], b"hello");
///     mem.data_mut(&mut *store)[offset..offset + 5].copy_from_slice(b"bye!!");
///
///     Ok(())
/// }
/// ```
///
/// It's worth also, however, covering some examples of **incorrect**,
/// **unsafe** usages of `Memory`. Do not do these things!
///
/// ```rust
/// # use anyhow::Result;
/// use wasmtime::{Memory, Store};
///
/// // NOTE: All code in this function is not safe to execute and may cause
/// // segfaults/undefined behavior at runtime. Do not copy/paste these examples
/// // into production code!
/// unsafe fn unsafe_examples(mem: Memory, store: &mut Store<()>) -> Result<()> {
///     // First and foremost, any borrow can be invalidated at any time via the
///     // `Memory::grow` function. This can relocate memory which causes any
///     // previous pointer to be possibly invalid now.
///     let pointer: &u8 = &*mem.data_ptr(&store);
///     mem.grow(&mut *store, 1)?; // invalidates `pointer`!
///     // println!("{}", *pointer); // FATAL: use-after-free
///
///     // Note that the use-after-free also applies to slices, whether they're
///     // slices of bytes or strings.
///     let mem_slice = std::slice::from_raw_parts(
///         mem.data_ptr(&store),
///         mem.data_size(&store),
///     );
///     let slice: &[u8] = &mem_slice[0x100..0x102];
///     mem.grow(&mut *store, 1)?; // invalidates `slice`!
///     // println!("{:?}", slice); // FATAL: use-after-free
///
///     // The `Memory` type may be stored in other locations, so if you hand
///     // off access to the `Store` then those locations may also call
///     // `Memory::grow` or similar, so it's not enough to just audit code for
///     // calls to `Memory::grow`.
///     let pointer: &u8 = &*mem.data_ptr(&store);
///     some_other_function(store); // may invalidate `pointer` through use of `store`
///     // println!("{:?}", pointer); // FATAL: maybe a use-after-free
///
///     // An especially subtle aspect of accessing a wasm instance's memory is
///     // that you need to be extremely careful about aliasing. Anyone at any
///     // time can call `data_unchecked()` or `data_unchecked_mut()`, which
///     // means you can easily have aliasing mutable references:
///     let ref1: &u8 = &*mem.data_ptr(&store).add(0x100);
///     let ref2: &mut u8 = &mut *mem.data_ptr(&store).add(0x100);
///     // *ref2 = *ref1; // FATAL: violates Rust's aliasing rules
///
///     Ok(())
/// }
/// # fn some_other_function(store: &mut Store<()>) {}
/// ```
///
/// Overall there's some general rules of thumb when unsafely working with
/// `Memory` and getting raw pointers inside of it:
///
/// * If you never have a "long lived" pointer into memory, you're likely in the
///   clear. Care still needs to be taken in threaded scenarios or when/where
///   data is read, but you'll be shielded from many classes of issues.
/// * Long-lived pointers must always respect Rust'a aliasing rules. It's ok for
///   shared borrows to overlap with each other, but mutable borrows must
///   overlap with nothing.
/// * Long-lived pointers are only valid if they're not invalidated for their
///   lifetime. This means that [`Store`](crate::Store) isn't used to reenter
///   wasm or the memory itself is never grown or otherwise modified/aliased.
///
/// At this point it's worth reiterating again that unsafely working with
/// `Memory` is pretty tricky and not recommended! It's highly recommended to
/// use the safe methods to interact with [`Memory`] whenever possible.
///
/// ## `Memory` Safety and Threads
///
/// Currently the `wasmtime` crate does not implement the wasm threads proposal,
/// but it is planned to do so. It may be interesting to readers to see how this
/// affects memory safety and what was previously just discussed as well.
///
/// Once threads are added into the mix, all of the above rules still apply.
/// There's an additional consideration that all reads and writes can happen
/// concurrently, though. This effectively means that any borrow into wasm
/// memory are virtually never safe to have.
///
/// Mutable pointers are fundamentally unsafe to have in a concurrent scenario
/// in the face of arbitrary wasm code. Only if you dynamically know for sure
/// that wasm won't access a region would it be safe to construct a mutable
/// pointer. Additionally even shared pointers are largely unsafe because their
/// underlying contents may change, so unless `UnsafeCell` in one form or
/// another is used everywhere there's no safety.
///
/// One important point about concurrency is that while [`Memory::grow`] can
/// happen concurrently it will never relocate the base pointer. Shared
/// memories must always have a maximum size and they will be preallocated such
/// that growth will never relocate the base pointer. The current size of the
/// memory may still change over time though.
///
/// Overall the general rule of thumb for shared memories is that you must
/// atomically read and write everything. Nothing can be borrowed and everything
/// must be eagerly copied out. This means that [`Memory::data`] and
/// [`Memory::data_mut`] won't work in the future (they'll probably return an
/// error) for shared memories when they're implemented. When possible it's
/// recommended to use [`Memory::read`] and [`Memory::write`] which will still
/// be provided.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)] // here for the C API
pub struct Memory(Stored<crate::runtime::vm::ExportMemory>);

impl Memory {
    /// Creates a new WebAssembly memory given the configuration of `ty`.
    ///
    /// The `store` argument will be the owner of the returned [`Memory`]. All
    /// WebAssembly memory is initialized to zero.
    ///
    /// # Panics
    ///
    /// This function will panic if the [`Store`](`crate::Store`) has a
    /// [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`) (see also:
    /// [`Store::limiter_async`](`crate::Store::limiter_async`)). When
    /// using an async resource limiter, use [`Memory::new_async`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// let engine = Engine::default();
    /// let mut store = Store::new(&engine, ());
    ///
    /// let memory_ty = MemoryType::new(1, None);
    /// let memory = Memory::new(&mut store, memory_ty)?;
    ///
    /// let module = Module::new(&engine, "(module (memory (import \"\" \"\") 1))")?;
    /// let instance = Instance::new(&mut store, &module, &[memory.into()])?;
    /// // ...
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(mut store: impl AsContextMut, ty: MemoryType) -> Result<Memory> {
        Self::_new(store.as_context_mut().0, ty)
    }

    /// Async variant of [`Memory::new`]. You must use this variant with
    /// [`Store`](`crate::Store`)s which have a
    /// [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`).
    ///
    /// # Panics
    ///
    /// This function will panic when used with a non-async
    /// [`Store`](`crate::Store`).
    #[cfg(feature = "async")]
    pub async fn new_async<T>(
        mut store: impl AsContextMut<Data = T>,
        ty: MemoryType,
    ) -> Result<Memory>
    where
        T: Send,
    {
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "cannot use `new_async` without enabling async support on the config"
        );
        store.on_fiber(|store| Self::_new(store.0, ty)).await?
    }

    /// Helper function for attaching the memory to a "frankenstein" instance
    fn _new(store: &mut StoreOpaque, ty: MemoryType) -> Result<Memory> {
        unsafe {
            let export = generate_memory_export(store, &ty, None)?;
            Ok(Memory::from_wasmtime_memory(export, store))
        }
    }

    /// Returns the underlying type of this memory.
    ///
    /// # Panics
    ///
    /// Panics if this memory doesn't belong to `store`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// let engine = Engine::default();
    /// let mut store = Store::new(&engine, ());
    /// let module = Module::new(&engine, "(module (memory (export \"mem\") 1))")?;
    /// let instance = Instance::new(&mut store, &module, &[])?;
    /// let memory = instance.get_memory(&mut store, "mem").unwrap();
    /// let ty = memory.ty(&store);
    /// assert_eq!(ty.minimum(), 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn ty(&self, store: impl AsContext) -> MemoryType {
        let store = store.as_context();
        let ty = &store[self.0].memory.memory;
        MemoryType::from_wasmtime_memory(&ty)
    }

    /// Safely reads memory contents at the given offset into a buffer.
    ///
    /// The entire buffer will be filled.
    ///
    /// If `offset + buffer.len()` exceed the current memory capacity, then the
    /// buffer is left untouched and a [`MemoryAccessError`] is returned.
    ///
    /// # Panics
    ///
    /// Panics if this memory doesn't belong to `store`.
    pub fn read(
        &self,
        store: impl AsContext,
        offset: usize,
        buffer: &mut [u8],
    ) -> Result<(), MemoryAccessError> {
        let store = store.as_context();
        let slice = self
            .data(&store)
            .get(offset..)
            .and_then(|s| s.get(..buffer.len()))
            .ok_or(MemoryAccessError { _private: () })?;
        buffer.copy_from_slice(slice);
        Ok(())
    }

    /// Safely writes contents of a buffer to this memory at the given offset.
    ///
    /// If the `offset + buffer.len()` exceeds the current memory capacity, then
    /// none of the buffer is written to memory and a [`MemoryAccessError`] is
    /// returned.
    ///
    /// # Panics
    ///
    /// Panics if this memory doesn't belong to `store`.
    pub fn write(
        &self,
        mut store: impl AsContextMut,
        offset: usize,
        buffer: &[u8],
    ) -> Result<(), MemoryAccessError> {
        let mut context = store.as_context_mut();
        self.data_mut(&mut context)
            .get_mut(offset..)
            .and_then(|s| s.get_mut(..buffer.len()))
            .ok_or(MemoryAccessError { _private: () })?
            .copy_from_slice(buffer);
        Ok(())
    }

    /// Returns this memory as a native Rust slice.
    ///
    /// Note that this method will consider the entire store context provided as
    /// borrowed for the duration of the lifetime of the returned slice.
    ///
    /// # Panics
    ///
    /// Panics if this memory doesn't belong to `store`.
    pub fn data<'a, T: 'a>(&self, store: impl Into<StoreContext<'a, T>>) -> &'a [u8] {
        unsafe {
            let store = store.into();
            let definition = &*store[self.0].definition;
            debug_assert!(!self.ty(store).is_shared());
            slice::from_raw_parts(definition.base, definition.current_length())
        }
    }

    /// Returns this memory as a native Rust mutable slice.
    ///
    /// Note that this method will consider the entire store context provided as
    /// borrowed for the duration of the lifetime of the returned slice.
    ///
    /// # Panics
    ///
    /// Panics if this memory doesn't belong to `store`.
    pub fn data_mut<'a, T: 'a>(&self, store: impl Into<StoreContextMut<'a, T>>) -> &'a mut [u8] {
        unsafe {
            let store = store.into();
            let definition = &*store[self.0].definition;
            debug_assert!(!self.ty(store).is_shared());
            slice::from_raw_parts_mut(definition.base, definition.current_length())
        }
    }

    /// Same as [`Memory::data_mut`], but also returns the `T` from the
    /// [`StoreContextMut`].
    ///
    /// This method can be used when you want to simultaneously work with the
    /// `T` in the store as well as the memory behind this [`Memory`]. Using
    /// [`Memory::data_mut`] would consider the entire store borrowed, whereas
    /// this method allows the Rust compiler to see that the borrow of this
    /// memory and the borrow of `T` are disjoint.
    ///
    /// # Panics
    ///
    /// Panics if this memory doesn't belong to `store`.
    pub fn data_and_store_mut<'a, T: 'a>(
        &self,
        store: impl Into<StoreContextMut<'a, T>>,
    ) -> (&'a mut [u8], &'a mut T) {
        // Note the unsafety here. Our goal is to simultaneously borrow the
        // memory and custom data from `store`, and the store it's connected
        // to. Rust will not let us do that, however, because we must call two
        // separate methods (both of which borrow the whole `store`) and one of
        // our borrows is mutable (the custom data).
        //
        // This operation, however, is safe because these borrows do not overlap
        // and in the process of borrowing them mutability doesn't actually
        // touch anything. This is akin to mutably borrowing two indices in an
        // array, which is safe so long as the indices are separate.
        unsafe {
            let mut store = store.into();
            let data = &mut *(store.data_mut() as *mut T);
            (self.data_mut(store), data)
        }
    }

    /// Returns the base pointer, in the host's address space, that the memory
    /// is located at.
    ///
    /// For more information and examples see the documentation on the
    /// [`Memory`] type.
    ///
    /// # Panics
    ///
    /// Panics if this memory doesn't belong to `store`.
    pub fn data_ptr(&self, store: impl AsContext) -> *mut u8 {
        unsafe { (*store.as_context()[self.0].definition).base }
    }

    /// Returns the byte length of this memory.
    ///
    /// WebAssembly memories are made up of a whole number of pages, so the byte
    /// size returned will always be a multiple of this memory's page size. Note
    /// that different Wasm memories may have different page sizes. You can get
    /// a memory's page size via the [`Memory::page_size`] method.
    ///
    /// By default the page size is 64KiB (aka `0x10000`, `2**16`, `1<<16`, or
    /// `65536`) but [the custom-page-sizes proposal] allows a memory to opt
    /// into a page size of `1`. Future extensions might allow any power of two
    /// as a page size.
    ///
    /// [the custom-page-sizes proposal]: https://github.com/WebAssembly/custom-page-sizes
    ///
    /// For more information and examples see the documentation on the
    /// [`Memory`] type.
    ///
    /// # Panics
    ///
    /// Panics if this memory doesn't belong to `store`.
    pub fn data_size(&self, store: impl AsContext) -> usize {
        self.internal_data_size(store.as_context().0)
    }

    pub(crate) fn internal_data_size(&self, store: &StoreOpaque) -> usize {
        unsafe { (*store[self.0].definition).current_length() }
    }

    /// Returns the size, in units of pages, of this Wasm memory.
    ///
    /// WebAssembly memories are made up of a whole number of pages, so the byte
    /// size returned will always be a multiple of this memory's page size. Note
    /// that different Wasm memories may have different page sizes. You can get
    /// a memory's page size via the [`Memory::page_size`] method.
    ///
    /// By default the page size is 64KiB (aka `0x10000`, `2**16`, `1<<16`, or
    /// `65536`) but [the custom-page-sizes proposal] allows a memory to opt
    /// into a page size of `1`. Future extensions might allow any power of two
    /// as a page size.
    ///
    /// [the custom-page-sizes proposal]: https://github.com/WebAssembly/custom-page-sizes
    ///
    /// # Panics
    ///
    /// Panics if this memory doesn't belong to `store`.
    pub fn size(&self, store: impl AsContext) -> u64 {
        self.internal_size(store.as_context().0)
    }

    pub(crate) fn internal_size(&self, store: &StoreOpaque) -> u64 {
        let byte_size = self.internal_data_size(store);
        let page_size = usize::try_from(self._page_size(store)).unwrap();
        u64::try_from(byte_size / page_size).unwrap()
    }

    /// Returns the size of a page, in bytes, for this memory.
    ///
    /// WebAssembly memories are made up of a whole number of pages, so the byte
    /// size (as returned by [`Memory::data_size`]) will always be a multiple of
    /// their page size. Different Wasm memories may have different page sizes.
    ///
    /// By default this is 64KiB (aka `0x10000`, `2**16`, `1<<16`, or `65536`)
    /// but [the custom-page-sizes proposal] allows opting into a page size of
    /// `1`. Future extensions might allow any power of two as a page size.
    ///
    /// [the custom-page-sizes proposal]: https://github.com/WebAssembly/custom-page-sizes
    pub fn page_size(&self, store: impl AsContext) -> u64 {
        self._page_size(store.as_context().0)
    }

    pub(crate) fn _page_size(&self, store: &StoreOpaque) -> u64 {
        store[self.0].memory.memory.page_size()
    }

    /// Returns the log2 of this memory's page size, in bytes.
    ///
    /// WebAssembly memories are made up of a whole number of pages, so the byte
    /// size (as returned by [`Memory::data_size`]) will always be a multiple of
    /// their page size. Different Wasm memories may have different page sizes.
    ///
    /// By default the page size is 64KiB (aka `0x10000`, `2**16`, `1<<16`, or
    /// `65536`) but [the custom-page-sizes proposal] allows opting into a page
    /// size of `1`. Future extensions might allow any power of two as a page
    /// size.
    ///
    /// [the custom-page-sizes proposal]: https://github.com/WebAssembly/custom-page-sizes
    pub fn page_size_log2(&self, store: impl AsContext) -> u8 {
        self._page_size_log2(store.as_context().0)
    }

    pub(crate) fn _page_size_log2(&self, store: &StoreOpaque) -> u8 {
        store[self.0].memory.memory.page_size_log2
    }

    /// Grows this WebAssembly memory by `delta` pages.
    ///
    /// This will attempt to add `delta` more pages of memory on to the end of
    /// this `Memory` instance. If successful this may relocate the memory and
    /// cause [`Memory::data_ptr`] to return a new value. Additionally any
    /// unsafely constructed slices into this memory may no longer be valid.
    ///
    /// On success returns the number of pages this memory previously had
    /// before the growth succeeded.
    ///
    /// Note that, by default, a WebAssembly memory's page size is 64KiB (aka
    /// 65536 or 2<sup>16</sup>). The [custom-page-sizes proposal] allows Wasm
    /// memories to opt into a page size of one byte (and this may be further
    /// relaxed to any power of two in a future extension).
    ///
    /// [custom-page-sizes proposal]: https://github.com/WebAssembly/custom-page-sizes
    ///
    /// # Errors
    ///
    /// Returns an error if memory could not be grown, for example if it exceeds
    /// the maximum limits of this memory. A
    /// [`ResourceLimiter`](crate::ResourceLimiter) is another example of
    /// preventing a memory to grow.
    ///
    /// # Panics
    ///
    /// Panics if this memory doesn't belong to `store`.
    ///
    /// This function will panic if the [`Store`](`crate::Store`) has a
    /// [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`) (see also:
    /// [`Store::limiter_async`](`crate::Store::limiter_async`). When using an
    /// async resource limiter, use [`Memory::grow_async`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// let engine = Engine::default();
    /// let mut store = Store::new(&engine, ());
    /// let module = Module::new(&engine, "(module (memory (export \"mem\") 1 2))")?;
    /// let instance = Instance::new(&mut store, &module, &[])?;
    /// let memory = instance.get_memory(&mut store, "mem").unwrap();
    ///
    /// assert_eq!(memory.size(&store), 1);
    /// assert_eq!(memory.grow(&mut store, 1)?, 1);
    /// assert_eq!(memory.size(&store), 2);
    /// assert!(memory.grow(&mut store, 1).is_err());
    /// assert_eq!(memory.size(&store), 2);
    /// assert_eq!(memory.grow(&mut store, 0)?, 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn grow(&self, mut store: impl AsContextMut, delta: u64) -> Result<u64> {
        let store = store.as_context_mut().0;
        let mem = self.wasmtime_memory(store);
        unsafe {
            match (*mem).grow(delta, Some(store))? {
                Some(size) => {
                    let vm = (*mem).vmmemory();
                    *store[self.0].definition = vm;
                    let page_size = (*mem).page_size();
                    Ok(u64::try_from(size).unwrap() / page_size)
                }
                None => bail!("failed to grow memory by `{}`", delta),
            }
        }
    }

    /// Async variant of [`Memory::grow`]. Required when using a
    /// [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`).
    ///
    /// # Panics
    ///
    /// This function will panic when used with a non-async
    /// [`Store`](`crate::Store`).
    #[cfg(feature = "async")]
    pub async fn grow_async<T>(
        &self,
        mut store: impl AsContextMut<Data = T>,
        delta: u64,
    ) -> Result<u64>
    where
        T: Send,
    {
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "cannot use `grow_async` without enabling async support on the config"
        );
        store.on_fiber(|store| self.grow(store, delta)).await?
    }

    fn wasmtime_memory(&self, store: &mut StoreOpaque) -> *mut crate::runtime::vm::Memory {
        unsafe {
            let export = &store[self.0];
            crate::runtime::vm::Instance::from_vmctx(export.vmctx, |handle| {
                handle.get_defined_memory(export.index)
            })
        }
    }

    pub(crate) unsafe fn from_wasmtime_memory(
        wasmtime_export: crate::runtime::vm::ExportMemory,
        store: &mut StoreOpaque,
    ) -> Memory {
        Memory(store.store_data_mut().insert(wasmtime_export))
    }

    pub(crate) fn wasmtime_ty<'a>(&self, store: &'a StoreData) -> &'a wasmtime_environ::Memory {
        &store[self.0].memory.memory
    }

    pub(crate) fn vmimport(&self, store: &StoreOpaque) -> crate::runtime::vm::VMMemoryImport {
        let export = &store[self.0];
        crate::runtime::vm::VMMemoryImport {
            from: export.definition,
            vmctx: export.vmctx,
            index: export.index,
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        store.store_data().contains(self.0)
    }

    /// Get a stable hash key for this memory.
    ///
    /// Even if the same underlying memory definition is added to the
    /// `StoreData` multiple times and becomes multiple `wasmtime::Memory`s,
    /// this hash key will be consistent across all of these memories.
    pub(crate) fn hash_key(&self, store: &StoreOpaque) -> impl core::hash::Hash + Eq {
        store[self.0].definition as usize
    }
}

/// A linear memory. This trait provides an interface for raw memory buffers
/// which are used by wasmtime, e.g. inside ['Memory']. Such buffers are in
/// principle not thread safe. By implementing this trait together with
/// MemoryCreator, one can supply wasmtime with custom allocated host managed
/// memory.
///
/// # Safety
///
/// The memory should be page aligned and a multiple of page size.
/// To prevent possible silent overflows, the memory should be protected by a
/// guard page.  Additionally the safety concerns explained in ['Memory'], for
/// accessing the memory apply here as well.
///
/// Note that this is a relatively new and experimental feature and it is
/// recommended to be familiar with wasmtime runtime code to use it.
pub unsafe trait LinearMemory: Send + Sync + 'static {
    /// Returns the number of allocated bytes which are accessible at this time.
    fn byte_size(&self) -> usize;

    /// Returns the maximum number of bytes the memory can grow to.
    ///
    /// Returns `None` if the memory is unbounded, or `Some` if memory cannot
    /// grow beyond a specified limit.
    fn maximum_byte_size(&self) -> Option<usize>;

    /// Grows this memory to have the `new_size`, in bytes, specified.
    ///
    /// Returns `Err` if memory can't be grown by the specified amount
    /// of bytes. The error may be downcastable to `std::io::Error`.
    /// Returns `Ok` if memory was grown successfully.
    fn grow_to(&mut self, new_size: usize) -> Result<()>;

    /// Return the allocated memory as a mutable pointer to u8.
    fn as_ptr(&self) -> *mut u8;

    /// Returns the range of native addresses that WebAssembly can natively
    /// access from this linear memory, including guard pages.
    fn wasm_accessible(&self) -> Range<usize>;
}

/// A memory creator. Can be used to provide a memory creator
/// to wasmtime which supplies host managed memory.
///
/// # Safety
///
/// This trait is unsafe, as the memory safety depends on proper implementation
/// of memory management. Memories created by the MemoryCreator should always be
/// treated as owned by wasmtime instance, and any modification of them outside
/// of wasmtime invoked routines is unsafe and may lead to corruption.
///
/// Note that this is a relatively new and experimental feature and it is
/// recommended to be familiar with wasmtime runtime code to use it.
pub unsafe trait MemoryCreator: Send + Sync {
    /// Create a new `LinearMemory` object from the specified parameters.
    ///
    /// The type of memory being created is specified by `ty` which indicates
    /// both the minimum and maximum size, in wasm pages. The minimum and
    /// maximum sizes, in bytes, are also specified as parameters to avoid
    /// integer conversion if desired.
    ///
    /// The `reserved_size_in_bytes` value indicates the expected size of the
    /// reservation that is to be made for this memory. If this value is `None`
    /// than the implementation is free to allocate memory as it sees fit. If
    /// the value is `Some`, however, then the implementation is expected to
    /// reserve that many bytes for the memory's allocation, plus the guard
    /// size at the end. Note that this reservation need only be a virtual
    /// memory reservation, physical memory does not need to be allocated
    /// immediately. In this case `grow` should never move the base pointer and
    /// the maximum size of `ty` is guaranteed to fit within
    /// `reserved_size_in_bytes`.
    ///
    /// The `guard_size_in_bytes` parameter indicates how many bytes of space,
    /// after the memory allocation, is expected to be unmapped. JIT code will
    /// elide bounds checks based on the `guard_size_in_bytes` provided, so for
    /// JIT code to work correctly the memory returned will need to be properly
    /// guarded with `guard_size_in_bytes` bytes left unmapped after the base
    /// allocation.
    ///
    /// Note that the `reserved_size_in_bytes` and `guard_size_in_bytes` options
    /// are tuned from the various [`Config`](crate::Config) methods about
    /// memory sizes/guards. Additionally these two values are guaranteed to be
    /// multiples of the system page size.
    ///
    /// Memory created from this method should be zero filled.
    fn new_memory(
        &self,
        ty: MemoryType,
        minimum: usize,
        maximum: Option<usize>,
        reserved_size_in_bytes: Option<usize>,
        guard_size_in_bytes: usize,
    ) -> Result<Box<dyn LinearMemory>, String>;
}

/// A constructor for externally-created shared memory.
///
/// The [threads proposal] adds the concept of "shared memory" to WebAssembly.
/// This is much the same as a Wasm linear memory (i.e., [`Memory`]), but can be
/// used concurrently by multiple agents. Because these agents may execute in
/// different threads, [`SharedMemory`] must be thread-safe.
///
/// When the threads proposal is enabled, there are multiple ways to construct
/// shared memory:
///  1. for imported shared memory, e.g., `(import "env" "memory" (memory 1 1
///     shared))`, the user must supply a [`SharedMemory`] with the
///     externally-created memory as an import to the instance--e.g.,
///     `shared_memory.into()`.
///  2. for private or exported shared memory, e.g., `(export "env" "memory"
///     (memory 1 1 shared))`, Wasmtime will create the memory internally during
///     instantiation--access using `Instance::get_shared_memory()`.
///
/// [threads proposal]:
///     https://github.com/WebAssembly/threads/blob/master/proposals/threads/Overview.md
///
/// # Examples
///
/// ```
/// # use wasmtime::*;
/// # fn main() -> anyhow::Result<()> {
/// let mut config = Config::new();
/// config.wasm_threads(true);
/// let engine = Engine::new(&config)?;
/// let mut store = Store::new(&engine, ());
///
/// let shared_memory = SharedMemory::new(&engine, MemoryType::shared(1, 2))?;
/// let module = Module::new(&engine, r#"(module (memory (import "" "") 1 2 shared))"#)?;
/// let instance = Instance::new(&mut store, &module, &[shared_memory.into()])?;
/// // ...
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct SharedMemory {
    vm: crate::runtime::vm::SharedMemory,
    engine: Engine,
    page_size_log2: u8,
}

impl SharedMemory {
    /// Construct a [`SharedMemory`] by providing both the `minimum` and
    /// `maximum` number of 64K-sized pages. This call allocates the necessary
    /// pages on the system.
    #[cfg(feature = "threads")]
    pub fn new(engine: &Engine, ty: MemoryType) -> Result<Self> {
        if !ty.is_shared() {
            bail!("shared memory must have the `shared` flag enabled on its memory type")
        }
        debug_assert!(ty.maximum().is_some());

        let tunables = engine.tunables();
        let plan = MemoryPlan::for_memory(*ty.wasmtime_memory(), tunables);
        let page_size_log2 = plan.memory.page_size_log2;
        let memory = crate::runtime::vm::SharedMemory::new(plan)?;

        Ok(Self {
            vm: memory,
            engine: engine.clone(),
            page_size_log2,
        })
    }

    /// Return the type of the shared memory.
    pub fn ty(&self) -> MemoryType {
        MemoryType::from_wasmtime_memory(&self.vm.ty())
    }

    /// Returns the size, in WebAssembly pages, of this wasm memory.
    pub fn size(&self) -> u64 {
        let byte_size = u64::try_from(self.data_size()).unwrap();
        let page_size = u64::from(self.page_size());
        byte_size / page_size
    }

    /// Returns the size of a page, in bytes, for this memory.
    ///
    /// By default this is 64KiB (aka `0x10000`, `2**16`, `1<<16`, or `65536`)
    /// but [the custom-page-sizes proposal] allows opting into a page size of
    /// `1`. Future extensions might allow any power of two as a page size.
    ///
    /// [the custom-page-sizes proposal]: https://github.com/WebAssembly/custom-page-sizes
    pub fn page_size(&self) -> u32 {
        debug_assert!(self.page_size_log2 == 0 || self.page_size_log2 == 16);
        1 << self.page_size_log2
    }

    /// Returns the byte length of this memory.
    ///
    /// The returned value will be a multiple of the wasm page size, 64k.
    ///
    /// For more information and examples see the documentation on the
    /// [`Memory`] type.
    pub fn data_size(&self) -> usize {
        self.vm.byte_size()
    }

    /// Return access to the available portion of the shared memory.
    ///
    /// The slice returned represents the region of accessible memory at the
    /// time that this function was called. The contents of the returned slice
    /// will reflect concurrent modifications happening on other threads.
    ///
    /// # Safety
    ///
    /// The returned slice is valid for the entire duration of the lifetime of
    /// this instance of [`SharedMemory`]. The base pointer of a shared memory
    /// does not change. This [`SharedMemory`] may grow further after this
    /// function has been called, but the slice returned will not grow.
    ///
    /// Concurrent modifications may be happening to the data returned on other
    /// threads. The `UnsafeCell<u8>` represents that safe access to the
    /// contents of the slice is not possible through normal loads and stores.
    ///
    /// The memory returned must be accessed safely through the `Atomic*` types
    /// in the [`std::sync::atomic`] module. Casting to those types must
    /// currently be done unsafely.
    pub fn data(&self) -> &[UnsafeCell<u8>] {
        unsafe {
            let definition = &*self.vm.vmmemory_ptr();
            slice::from_raw_parts(definition.base.cast(), definition.current_length())
        }
    }

    /// Grows this WebAssembly memory by `delta` pages.
    ///
    /// This will attempt to add `delta` more pages of memory on to the end of
    /// this `Memory` instance. If successful this may relocate the memory and
    /// cause [`Memory::data_ptr`] to return a new value. Additionally any
    /// unsafely constructed slices into this memory may no longer be valid.
    ///
    /// On success returns the number of pages this memory previously had
    /// before the growth succeeded.
    ///
    /// # Errors
    ///
    /// Returns an error if memory could not be grown, for example if it exceeds
    /// the maximum limits of this memory. A
    /// [`ResourceLimiter`](crate::ResourceLimiter) is another example of
    /// preventing a memory to grow.
    pub fn grow(&self, delta: u64) -> Result<u64> {
        match self.vm.grow(delta, None)? {
            Some((old_size, _new_size)) => {
                // For shared memory, the `VMMemoryDefinition` is updated inside
                // the locked region.
                Ok(u64::try_from(old_size).unwrap() / u64::from(self.page_size()))
            }
            None => bail!("failed to grow memory by `{}`", delta),
        }
    }

    /// Equivalent of the WebAssembly `memory.atomic.notify` instruction for
    /// this shared memory.
    ///
    /// This method allows embedders to notify threads blocked on the specified
    /// `addr`, an index into wasm linear memory. Threads could include
    /// wasm threads blocked on a `memory.atomic.wait*` instruction or embedder
    /// threads blocked on [`SharedMemory::atomic_wait32`], for example.
    ///
    /// The `count` argument is the number of threads to wake up.
    ///
    /// This function returns the number of threads awoken.
    ///
    /// # Errors
    ///
    /// This function will return an error if `addr` is not within bounds or
    /// not aligned to a 4-byte boundary.
    pub fn atomic_notify(&self, addr: u64, count: u32) -> Result<u32, Trap> {
        self.vm.atomic_notify(addr, count)
    }

    /// Equivalent of the WebAssembly `memory.atomic.wait32` instruction for
    /// this shared memory.
    ///
    /// This method allows embedders to block the current thread until notified
    /// via the `memory.atomic.notify` instruction or the
    /// [`SharedMemory::atomic_notify`] method, enabling synchronization with
    /// the wasm guest as desired.
    ///
    /// The `expected` argument is the expected 32-bit value to be stored at
    /// the byte address `addr` specified. The `addr` specified is an index
    /// into this linear memory.
    ///
    /// The optional `timeout` argument is the maximum amount of time to block
    /// the current thread. If not specified the thread may sleep indefinitely.
    ///
    /// This function returns one of three possible values:
    ///
    /// * `WaitResult::Ok` - this function, loaded the value at `addr`, found
    ///   it was equal to `expected`, and then blocked (all as one atomic
    ///   operation). The thread was then awoken with a `memory.atomic.notify`
    ///   instruction or the [`SharedMemory::atomic_notify`] method.
    /// * `WaitResult::Mismatch` - the value at `addr` was loaded but was not
    ///   equal to `expected` so the thread did not block and immediately
    ///   returned.
    /// * `WaitResult::TimedOut` - all the steps of `Ok` happened, except this
    ///   thread was woken up due to a timeout.
    ///
    /// This function will not return due to spurious wakeups.
    ///
    /// # Errors
    ///
    /// This function will return an error if `addr` is not within bounds or
    /// not aligned to a 4-byte boundary.
    pub fn atomic_wait32(
        &self,
        addr: u64,
        expected: u32,
        timeout: Option<Duration>,
    ) -> Result<WaitResult, Trap> {
        self.vm.atomic_wait32(addr, expected, timeout)
    }

    /// Equivalent of the WebAssembly `memory.atomic.wait64` instruction for
    /// this shared memory.
    ///
    /// For more information see [`SharedMemory::atomic_wait32`].
    ///
    /// # Errors
    ///
    /// Returns the same error as [`SharedMemory::atomic_wait32`] except that
    /// the specified address must be 8-byte aligned instead of 4-byte aligned.
    pub fn atomic_wait64(
        &self,
        addr: u64,
        expected: u64,
        timeout: Option<Duration>,
    ) -> Result<WaitResult, Trap> {
        self.vm.atomic_wait64(addr, expected, timeout)
    }

    /// Return a reference to the [`Engine`] used to configure the shared
    /// memory.
    pub(crate) fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Construct a single-memory instance to provide a way to import
    /// [`SharedMemory`] into other modules.
    pub(crate) fn vmimport(&self, store: &mut StoreOpaque) -> crate::runtime::vm::VMMemoryImport {
        let export_memory = generate_memory_export(store, &self.ty(), Some(&self.vm)).unwrap();
        VMMemoryImport {
            from: export_memory.definition,
            vmctx: export_memory.vmctx,
            index: export_memory.index,
        }
    }

    /// Create a [`SharedMemory`] from an [`ExportMemory`] definition. This
    /// function is available to handle the case in which a Wasm module exports
    /// shared memory and the user wants host-side access to it.
    pub(crate) unsafe fn from_wasmtime_memory(
        wasmtime_export: crate::runtime::vm::ExportMemory,
        store: &mut StoreOpaque,
    ) -> Self {
        #[cfg_attr(not(feature = "threads"), allow(unused_variables, unreachable_code))]
        crate::runtime::vm::Instance::from_vmctx(wasmtime_export.vmctx, |handle| {
            let memory_index = handle.module().memory_index(wasmtime_export.index);
            let page_size = handle.memory_page_size(memory_index);
            debug_assert!(page_size.is_power_of_two());
            let page_size_log2 = u8::try_from(page_size.ilog2()).unwrap();

            let memory = handle
                .get_defined_memory(wasmtime_export.index)
                .as_mut()
                .unwrap();
            match memory.as_shared_memory() {
                Some(mem) => Self {
                    vm: mem.clone(),
                    engine: store.engine().clone(),
                    page_size_log2,
                },
                None => panic!("unable to convert from a shared memory"),
            }
        })
    }
}

impl fmt::Debug for SharedMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedMemory").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    // Assert that creating a memory via `Memory::new` respects the limits/tunables
    // in `Config`.
    #[test]
    fn respect_tunables() {
        let mut cfg = Config::new();
        cfg.static_memory_maximum_size(0)
            .dynamic_memory_guard_size(0);
        let mut store = Store::new(&Engine::new(&cfg).unwrap(), ());
        let ty = MemoryType::new(1, None);
        let mem = Memory::new(&mut store, ty).unwrap();
        let store = store.as_context();
        assert_eq!(store[mem.0].memory.offset_guard_size, 0);
        match &store[mem.0].memory.style {
            wasmtime_environ::MemoryStyle::Dynamic { .. } => {}
            other => panic!("unexpected style {:?}", other),
        }
    }

    #[test]
    fn hash_key_is_stable_across_duplicate_store_data_entries() -> Result<()> {
        let mut store = Store::<()>::default();
        let module = Module::new(
            store.engine(),
            r#"
                (module
                    (memory (export "m") 1 1)
                )
            "#,
        )?;
        let instance = Instance::new(&mut store, &module, &[])?;

        // Each time we `get_memory`, we call `Memory::from_wasmtime` which adds
        // a new entry to `StoreData`, so `g1` and `g2` will have different
        // indices into `StoreData`.
        let m1 = instance.get_memory(&mut store, "m").unwrap();
        let m2 = instance.get_memory(&mut store, "m").unwrap();

        // That said, they really point to the same memory.
        assert_eq!(m1.data(&store)[0], 0);
        assert_eq!(m2.data(&store)[0], 0);
        m1.data_mut(&mut store)[0] = 42;
        assert_eq!(m1.data(&mut store)[0], 42);
        assert_eq!(m2.data(&mut store)[0], 42);

        // And therefore their hash keys are the same.
        assert!(m1.hash_key(&store.as_context().0) == m2.hash_key(&store.as_context().0));

        // But the hash keys are different from different memories.
        let instance2 = Instance::new(&mut store, &module, &[])?;
        let m3 = instance2.get_memory(&mut store, "m").unwrap();
        assert!(m1.hash_key(&store.as_context().0) != m3.hash_key(&store.as_context().0));

        Ok(())
    }
}
