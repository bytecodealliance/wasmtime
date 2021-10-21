use crate::store::{StoreData, StoreOpaque, Stored};
use crate::trampoline::generate_memory_export;
use crate::{AsContext, AsContextMut, MemoryType, StoreContext, StoreContextMut};
use anyhow::{bail, Result};
use std::convert::TryFrom;
use std::slice;

/// Error for out of bounds [`Memory`] access.
#[derive(Debug)]
#[non_exhaustive]
pub struct MemoryAccessError {
    // Keep struct internals private for future extensibility.
    _private: (),
}

impl std::fmt::Display for MemoryAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "out of bounds memory access")
    }
}

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
pub struct Memory(Stored<wasmtime_runtime::ExportMemory>);

impl Memory {
    /// Creates a new WebAssembly memory given the configuration of `ty`.
    ///
    /// The `store` argument will be the owner of the returned [`Memory`]. All
    /// WebAssembly memory is initialized to zero.
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
        Memory::_new(store.as_context_mut().0, ty)
    }

    /// Async variant of [`Memory::new`]. You must use this variant with [`Store`]s which have a
    /// [`ResourceLimiterAsync`].
    ///
    /// # Panics
    ///
    /// This function will panic when used with a non-async [`Store`].
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
        store.on_fiber(|store| Memory::_new(store.0, ty)).await?
    }

    fn _new(store: &mut StoreOpaque, ty: MemoryType) -> Result<Memory> {
        unsafe {
            let export = generate_memory_export(store, &ty)?;
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
            let definition = *store[self.0].definition;
            slice::from_raw_parts(definition.base, definition.current_length)
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
            let definition = *store[self.0].definition;
            slice::from_raw_parts_mut(definition.base, definition.current_length)
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
    /// The returned value will be a multiple of the wasm page size, 64k.
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
        unsafe { (*store[self.0].definition).current_length }
    }

    /// Returns the size, in WebAssembly pages, of this wasm memory.
    ///
    /// # Panics
    ///
    /// Panics if this memory doesn't belong to `store`.
    pub fn size(&self, store: impl AsContext) -> u64 {
        self.internal_size(store.as_context().0)
    }

    pub(crate) fn internal_size(&self, store: &StoreOpaque) -> u64 {
        (self.internal_data_size(store) / wasmtime_environ::WASM_PAGE_SIZE as usize) as u64
    }

    /// Grows this WebAssembly memory by `delta` pages.
    ///
    /// This will attempt to add `delta` more pages of memory on to the end of
    /// this `Memory` instance. If successful this may relocate the memory and
    /// cause [`Memory::data_ptr`] to return a new value. Additionally any
    /// unsafetly constructed slices into this memory may no longer be valid.
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
            match (*mem).grow(delta, store)? {
                Some(size) => {
                    let vm = (*mem).vmmemory();
                    *store[self.0].definition = vm;
                    Ok(u64::try_from(size).unwrap() / u64::from(wasmtime_environ::WASM_PAGE_SIZE))
                }
                None => bail!("failed to grow memory by `{}`", delta),
            }
        }
    }

    /// Async variant of [`Memory::grow`]. Required when using a [`ResourceLimiterAsync`].
    ///
    /// # Panics
    ///
    /// This function will panic when used with a non-async [`Store`].
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
    fn wasmtime_memory(&self, store: &mut StoreOpaque) -> *mut wasmtime_runtime::Memory {
        unsafe {
            let export = &store[self.0];
            let mut handle = wasmtime_runtime::InstanceHandle::from_vmctx(export.vmctx);
            let idx = handle.memory_index(&*export.definition);
            handle.get_defined_memory(idx)
        }
    }

    pub(crate) unsafe fn from_wasmtime_memory(
        wasmtime_export: wasmtime_runtime::ExportMemory,
        store: &mut StoreOpaque,
    ) -> Memory {
        Memory(store.store_data_mut().insert(wasmtime_export))
    }

    pub(crate) fn wasmtime_ty<'a>(&self, store: &'a StoreData) -> &'a wasmtime_environ::Memory {
        &store[self.0].memory.memory
    }

    pub(crate) fn vmimport(&self, store: &StoreOpaque) -> wasmtime_runtime::VMMemoryImport {
        let export = &store[self.0];
        wasmtime_runtime::VMMemoryImport {
            from: export.definition,
            vmctx: export.vmctx,
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        store.store_data().contains(self.0)
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
    fn new_memory(
        &self,
        ty: MemoryType,
        minimum: usize,
        maximum: Option<usize>,
        reserved_size_in_bytes: Option<usize>,
        guard_size_in_bytes: usize,
    ) -> Result<Box<dyn LinearMemory>, String>;
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
}
