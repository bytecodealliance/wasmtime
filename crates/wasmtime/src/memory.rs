use crate::trampoline::{generate_memory_export, StoreInstanceHandle};
use crate::{MemoryType, Store};
use anyhow::{anyhow, Result};
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
/// WebAssembly memory is used for global data, statics in C/C++/Rust, shadow
/// stack memory, etc. Accessing wasm memory is generally quite fast!
///
/// # `Memory` and `Clone`
///
/// Memories are internally reference counted so you can `clone` a `Memory`. The
/// cloning process only performs a shallow clone, so two cloned `Memory`
/// instances are equivalent in their functionality.
///
/// # `Memory` and threads
///
/// It is intended that `Memory` is safe to share between threads. At this time
/// this is not implemented in `wasmtime`, however. This is planned to be
/// implemented though!
///
/// # `Memory` and Safety
///
/// Linear memory is a lynchpin of safety for WebAssembly, but it turns out
/// there are very few ways to safely inspect the contents of a memory from the
/// host (Rust). This is because memory safety is quite tricky when working with
/// a `Memory` and we're still working out the best idioms to encapsulate
/// everything safely where it's efficient and ergonomic. This section of
/// documentation, however, is intended to help educate a bit what is and isn't
/// safe when working with `Memory`.
///
/// For safety purposes you can think of a `Memory` as a glorified
/// `Rc<UnsafeCell<Vec<u8>>>`. There are a few consequences of this
/// interpretation:
///
/// * At any time someone else may have access to the memory (hence the `Rc`).
///   This could be a wasm instance, other host code, or a set of wasm instances
///   which all reference a `Memory`. When in doubt assume someone else has a
///   handle to your `Memory`.
///
/// * At any time, memory can be read from or written to (hence the
///   `UnsafeCell`). Anyone with a handle to a wasm memory can read/write to it.
///   Primarily other instances can execute the `load` and `store` family of
///   instructions, as well as any other which modifies or reads memory.
///
/// * At any time memory may grow (hence the `Vec<..>`). Growth may relocate the
///   base memory pointer (similar to how `vec.push(...)` can change the result
///   of `.as_ptr()`)
///
/// So given that we're working roughly with `Rc<UnsafeCell<Vec<u8>>>` that's a
/// lot to keep in mind! It's hopefully though sort of setting the stage as to
/// what you can safely do with memories.
///
/// Let's run through a few safe examples first of how you can use a `Memory`.
///
/// ```rust
/// use wasmtime::{Memory, MemoryAccessError};
///
/// // Memory can be read and written safely with the `Memory::read` and
/// // `Memory::write` methods.
/// // An error is returned if the copy did not succeed.
/// fn safe_examples(mem: &Memory) -> Result<(), MemoryAccessError> {
///     let offset = 5;
///     mem.write(offset, b"hello")?;
///     let mut buffer = [0u8; 5];
///     mem.read(offset, &mut buffer)?;
///     assert_eq!(b"hello", &buffer);
///     Ok(())
/// }
///
/// // You can also get direct, unsafe access to the memory, but must manually
/// // ensure that safety invariants are upheld.
///
/// fn correct_unsafe_examples(mem: &Memory) {
///     // Just like wasm, it's safe to read memory almost at any time. The
///     // gotcha here is that we need to be sure to load from the correct base
///     // pointer and perform the bounds check correctly. So long as this is
///     // all self contained here (e.g. not arbitrary code in the middle) we're
///     // good to go.
///     let byte = unsafe { mem.data_unchecked()[0x123] };
///
///     // Short-lived borrows of memory are safe, but they must be scoped and
///     // not have code which modifies/etc `Memory` while the borrow is active.
///     // For example if you want to read a string from memory it is safe to do
///     // so:
///     let string_base = 0xdead;
///     let string_len = 0xbeef;
///     let string = unsafe {
///         let bytes = &mem.data_unchecked()[string_base..][..string_len];
///         match std::str::from_utf8(bytes) {
///             Ok(s) => s.to_string(), // copy out of wasm memory
///             Err(_) => panic!("not valid utf-8"),
///         }
///     };
///
///     // Additionally like wasm you can write to memory at any point in time,
///     // again making sure that after you get the unchecked slice you don't
///     // execute code which could read/write/modify `Memory`:
///     unsafe {
///         mem.data_unchecked_mut()[0x123] = 3;
///     }
///
///     // When working with *borrows* that point directly into wasm memory you
///     // need to be extremely careful. Any functionality that operates on a
///     // borrow into wasm memory needs to be thoroughly audited to effectively
///     // not touch the `Memory` at all
///     let data_base = 0xfeed;
///     let data_len = 0xface;
///     unsafe {
///         let data = &mem.data_unchecked()[data_base..][..data_len];
///         host_function_that_doesnt_touch_memory(data);
///
///         // effectively the same rules apply to mutable borrows
///         let data_mut = &mut mem.data_unchecked_mut()[data_base..][..data_len];
///         host_function_that_doesnt_touch_memory(data);
///     }
/// }
/// # fn host_function_that_doesnt_touch_memory(_: &[u8]){}
/// ```
///
/// It's worth also, however, covering some examples of **incorrect**,
/// **unsafe** usages of `Memory`. Do not do these things!
///
/// ```rust
/// # use anyhow::Result;
/// use wasmtime::Memory;
///
/// // NOTE: All code in this function is not safe to execute and may cause
/// // segfaults/undefined behavior at runtime. Do not copy/paste these examples
/// // into production code!
/// unsafe fn unsafe_examples(mem: &Memory) -> Result<()> {
///     // First and foremost, any borrow can be invalidated at any time via the
///     // `Memory::grow` function. This can relocate memory which causes any
///     // previous pointer to be possibly invalid now.
///     let pointer: &u8 = &mem.data_unchecked()[0x100];
///     mem.grow(1)?; // invalidates `pointer`!
///     // println!("{}", *pointer); // FATAL: use-after-free
///
///     // Note that the use-after-free also applies to slices, whether they're
///     // slices of bytes or strings.
///     let slice: &[u8] = &mem.data_unchecked()[0x100..0x102];
///     mem.grow(1)?; // invalidates `slice`!
///     // println!("{:?}", slice); // FATAL: use-after-free
///
///     // Due to the reference-counted nature of `Memory` note that literal
///     // calls to `Memory::grow` are not sufficient to audit for. You'll need
///     // to be careful that any mutation of `Memory` doesn't happen while
///     // you're holding an active borrow.
///     let slice: &[u8] = &mem.data_unchecked()[0x100..0x102];
///     some_other_function(); // may invalidate `slice` through another `mem` reference
///     // println!("{:?}", slice); // FATAL: maybe a use-after-free
///
///     // An especially subtle aspect of accessing a wasm instance's memory is
///     // that you need to be extremely careful about aliasing. Anyone at any
///     // time can call `data_unchecked()` or `data_unchecked_mut()`, which
///     // means you can easily have aliasing mutable references:
///     let ref1: &u8 = &mem.data_unchecked()[0x100];
///     let ref2: &mut u8 = &mut mem.data_unchecked_mut()[0x100];
///     // *ref2 = *ref1; // FATAL: violates Rust's aliasing rules
///
///     // Note that aliasing applies to strings as well, for example this is
///     // not valid because the slices overlap.
///     let slice1: &mut [u8] = &mut mem.data_unchecked_mut()[0x100..][..3];
///     let slice2: &mut [u8] = &mut mem.data_unchecked_mut()[0x102..][..4];
///     // println!("{:?} {:?}", slice1, slice2); // FATAL: aliasing mutable pointers
///
///     Ok(())
/// }
/// # fn some_other_function() {}
/// ```
///
/// Overall there's some general rules of thumb when working with `Memory` and
/// getting raw pointers inside of it:
///
/// * If you never have a "long lived" pointer into memory, you're likely in the
///   clear. Care still needs to be taken in threaded scenarios or when/where
///   data is read, but you'll be shielded from many classes of issues.
/// * Long-lived pointers must always respect Rust'a aliasing rules. It's ok for
///   shared borrows to overlap with each other, but mutable borrows must
///   overlap with nothing.
/// * Long-lived pointers are only valid if `Memory` isn't used in an unsafe way
///   while the pointer is valid. This includes both aliasing and growth.
///
/// At this point it's worth reiterating again that working with `Memory` is
/// pretty tricky and that's not great! Proposals such as [interface types] are
/// intended to prevent wasm modules from even needing to import/export memory
/// in the first place, which obviates the need for all of these safety caveats!
/// Additionally over time we're still working out the best idioms to expose in
/// `wasmtime`, so if you've got ideas or questions please feel free to [open an
/// issue]!
///
/// ## `Memory` Safety and Threads
///
/// Currently the `wasmtime` crate does not implement the wasm threads proposal,
/// but it is planned to do so. It's additionally worthwhile discussing how this
/// affects memory safety and what was previously just discussed as well.
///
/// Once threads are added into the mix, all of the above rules still apply.
/// There's an additional, rule, however, that all reads and writes can
/// happen *concurrently*. This effectively means that long-lived borrows into
/// wasm memory are virtually never safe to have.
///
/// Mutable pointers are fundamentally unsafe to have in a concurrent scenario
/// in the face of arbitrary wasm code. Only if you dynamically know for sure
/// that wasm won't access a region would it be safe to construct a mutable
/// pointer. Additionally even shared pointers are largely unsafe because their
/// underlying contents may change, so unless `UnsafeCell` in one form or
/// another is used everywhere there's no safety.
///
/// One important point about concurrency is that `Memory::grow` can indeed
/// happen concurrently. This, however, will never relocate the base pointer.
/// Shared memories must always have a maximum size and they will be
/// preallocated such that growth will never relocate the base pointer. The
/// maximum length of the memory, however, will change over time.
///
/// Overall the general rule of thumb for shared memories is that you must
/// atomically read and write everything. Nothing can be borrowed and everything
/// must be eagerly copied out.
///
/// [interface types]: https://github.com/webassembly/interface-types
/// [open an issue]: https://github.com/bytecodealliance/wasmtime/issues/new
#[derive(Clone)]
pub struct Memory {
    pub(crate) instance: StoreInstanceHandle,
    wasmtime_export: wasmtime_runtime::ExportMemory,
}

impl Memory {
    /// Creates a new WebAssembly memory given the configuration of `ty`.
    ///
    /// The `store` argument is a general location for cache information, and
    /// otherwise the memory will immediately be allocated according to the
    /// type's configuration. All WebAssembly memory is initialized to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// let engine = Engine::default();
    /// let store = Store::new(&engine);
    ///
    /// let memory_ty = MemoryType::new(Limits::new(1, None));
    /// let memory = Memory::new(&store, memory_ty);
    ///
    /// let module = Module::new(&engine, "(module (memory (import \"\" \"\") 1))")?;
    /// let instance = Instance::new(&store, &module, &[memory.into()])?;
    /// // ...
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(store: &Store, ty: MemoryType) -> Memory {
        let (instance, wasmtime_export) =
            generate_memory_export(store, &ty).expect("generated memory");
        Memory {
            instance,
            wasmtime_export,
        }
    }

    /// Returns the underlying type of this memory.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// let engine = Engine::default();
    /// let store = Store::new(&engine);
    /// let module = Module::new(&engine, "(module (memory (export \"mem\") 1))")?;
    /// let instance = Instance::new(&store, &module, &[])?;
    /// let memory = instance.get_memory("mem").unwrap();
    /// let ty = memory.ty();
    /// assert_eq!(ty.limits().min(), 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn ty(&self) -> MemoryType {
        MemoryType::from_wasmtime_memory(&self.wasmtime_export.memory.memory)
    }

    /// Safely reads memory contents at the given offset into a buffer.
    ///
    /// The entire buffer will be filled.
    ///
    /// If offset + buffer length exceed the current memory capacity, then the
    /// buffer is left untouched and a [`MemoryAccessError`] is returned.
    pub fn read(&self, offset: usize, buffer: &mut [u8]) -> Result<(), MemoryAccessError> {
        unsafe {
            let slice = self
                .data_unchecked()
                .get(offset..)
                .and_then(|s| s.get(..buffer.len()))
                .ok_or(MemoryAccessError { _private: () })?;
            buffer.copy_from_slice(slice);
            Ok(())
        }
    }

    /// Safely writes contents of a buffer to this memory at the given offset.
    ///
    /// If the offset + buffer length exceed current memory capacity, then none
    /// of the buffer is written to memory and a [`MemoryAccessError`] is
    /// returned.
    pub fn write(&self, offset: usize, buffer: &[u8]) -> Result<(), MemoryAccessError> {
        unsafe {
            self.data_unchecked_mut()
                .get_mut(offset..)
                .and_then(|s| s.get_mut(..buffer.len()))
                .ok_or(MemoryAccessError { _private: () })?
                .copy_from_slice(buffer);
            Ok(())
        }
    }

    /// Returns this memory as a slice view that can be read natively in Rust.
    ///
    /// # Safety
    ///
    /// This is an unsafe operation because there is no guarantee that the
    /// following operations do not happen concurrently while the slice is in
    /// use:
    ///
    /// * Data could be modified by calling into a wasm module.
    /// * Memory could be relocated through growth by calling into a wasm
    ///   module.
    /// * When threads are supported, non-atomic reads will race with other
    ///   writes.
    ///
    /// Extreme care need be taken when the data of a `Memory` is read. The
    /// above invariants all need to be upheld at a bare minimum, and in
    /// general you'll need to ensure that while you're looking at slice you're
    /// the only one who can possibly look at the slice and read/write it.
    ///
    /// Be sure to keep in mind that `Memory` is reference counted, meaning
    /// that there may be other users of this `Memory` instance elsewhere in
    /// your program. Additionally `Memory` can be shared and used in any number
    /// of wasm instances, so calling any wasm code should be considered
    /// dangerous while you're holding a slice of memory.
    ///
    /// For more information and examples see the documentation on the
    /// [`Memory`] type.
    pub unsafe fn data_unchecked(&self) -> &[u8] {
        self.data_unchecked_mut()
    }

    /// Returns this memory as a slice view that can be read and written
    /// natively in Rust.
    ///
    /// # Safety
    ///
    /// All of the same safety caveats of [`Memory::data_unchecked`] apply
    /// here, doubly so because this is returning a mutable slice! As a
    /// double-extra reminder, remember that `Memory` is reference counted, so
    /// you can very easily acquire two mutable slices by simply calling this
    /// function twice. Extreme caution should be used when using this method,
    /// and in general you probably want to result to unsafe accessors and the
    /// `data` methods below.
    ///
    /// For more information and examples see the documentation on the
    /// [`Memory`] type.
    pub unsafe fn data_unchecked_mut(&self) -> &mut [u8] {
        let definition = &*self.wasmtime_export.definition;
        slice::from_raw_parts_mut(definition.base, definition.current_length)
    }

    /// Returns the base pointer, in the host's address space, that the memory
    /// is located at.
    ///
    /// When reading and manipulating memory be sure to read up on the caveats
    /// of [`Memory::data_unchecked`] to make sure that you can safely
    /// read/write the memory.
    ///
    /// For more information and examples see the documentation on the
    /// [`Memory`] type.
    pub fn data_ptr(&self) -> *mut u8 {
        unsafe { (*self.wasmtime_export.definition).base }
    }

    /// Returns the byte length of this memory.
    ///
    /// The returned value will be a multiple of the wasm page size, 64k.
    ///
    /// For more information and examples see the documentation on the
    /// [`Memory`] type.
    pub fn data_size(&self) -> usize {
        unsafe { (*self.wasmtime_export.definition).current_length }
    }

    /// Returns the size, in pages, of this wasm memory.
    pub fn size(&self) -> u32 {
        (self.data_size() / wasmtime_environ::WASM_PAGE_SIZE as usize) as u32
    }

    /// Grows this WebAssembly memory by `delta` pages.
    ///
    /// This will attempt to add `delta` more pages of memory on to the end of
    /// this `Memory` instance. If successful this may relocate the memory and
    /// cause [`Memory::data_ptr`] to return a new value. Additionally previous
    /// slices into this memory may no longer be valid.
    ///
    /// On success returns the number of pages this memory previously had
    /// before the growth succeeded.
    ///
    /// # Errors
    ///
    /// Returns an error if memory could not be grown, for example if it exceeds
    /// the maximum limits of this memory.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// let engine = Engine::default();
    /// let store = Store::new(&engine);
    /// let module = Module::new(&engine, "(module (memory (export \"mem\") 1 2))")?;
    /// let instance = Instance::new(&store, &module, &[])?;
    /// let memory = instance.get_memory("mem").unwrap();
    ///
    /// assert_eq!(memory.size(), 1);
    /// assert_eq!(memory.grow(1)?, 1);
    /// assert_eq!(memory.size(), 2);
    /// assert!(memory.grow(1).is_err());
    /// assert_eq!(memory.size(), 2);
    /// assert_eq!(memory.grow(0)?, 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn grow(&self, delta: u32) -> Result<u32> {
        let index = self
            .instance
            .memory_index(unsafe { &*self.wasmtime_export.definition });
        self.instance
            .memory_grow(index, delta)
            .ok_or_else(|| anyhow!("failed to grow memory"))
    }

    pub(crate) unsafe fn from_wasmtime_memory(
        wasmtime_export: &wasmtime_runtime::ExportMemory,
        store: &Store,
    ) -> Memory {
        Memory {
            instance: store.existing_vmctx(wasmtime_export.vmctx),
            wasmtime_export: wasmtime_export.clone(),
        }
    }

    pub(crate) fn wasmtime_ty(&self) -> &wasmtime_environ::wasm::Memory {
        &self.wasmtime_export.memory.memory
    }

    pub(crate) fn vmimport(&self) -> wasmtime_runtime::VMMemoryImport {
        wasmtime_runtime::VMMemoryImport {
            from: self.wasmtime_export.definition,
            vmctx: self.wasmtime_export.vmctx,
        }
    }

    pub(crate) fn wasmtime_export(&self) -> &wasmtime_runtime::ExportMemory {
        &self.wasmtime_export
    }
}

/// A linear memory. This trait provides an interface for raw memory buffers which are used
/// by wasmtime, e.g. inside ['Memory']. Such buffers are in principle not thread safe.
/// By implementing this trait together with MemoryCreator,
/// one can supply wasmtime with custom allocated host managed memory.
///
/// # Safety
/// The memory should be page aligned and a multiple of page size.
/// To prevent possible silent overflows, the memory should be protected by a guard page.
/// Additionally the safety concerns explained in ['Memory'], for accessing the memory
/// apply here as well.
///
/// Note that this is a relatively new and experimental feature and it is recommended
/// to be familiar with wasmtime runtime code to use it.
pub unsafe trait LinearMemory {
    /// Returns the number of allocated wasm pages.
    fn size(&self) -> u32;

    /// Grow memory by the specified amount of wasm pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of wasm pages.
    fn grow(&self, delta: u32) -> Option<u32>;

    /// Return the allocated memory as a mutable pointer to u8.
    fn as_ptr(&self) -> *mut u8;
}

/// A memory creator. Can be used to provide a memory creator
/// to wasmtime which supplies host managed memory.
///
/// # Safety
/// This trait is unsafe, as the memory safety depends on proper implementation of
/// memory management. Memories created by the MemoryCreator should always be treated
/// as owned by wasmtime instance, and any modification of them outside of wasmtime
/// invoked routines is unsafe and may lead to corruption.
///
/// Note that this is a relatively new and experimental feature and it is recommended
/// to be familiar with wasmtime runtime code to use it.
pub unsafe trait MemoryCreator: Send + Sync {
    /// Create a new `LinearMemory` object from the specified parameters.
    ///
    /// The type of memory being created is specified by `ty` which indicates
    /// both the minimum and maximum size, in wasm pages.
    ///
    /// The `reserved_size_in_bytes` value indicates the expected size of the
    /// reservation that is to be made for this memory. If this value is `None`
    /// than the implementation is free to allocate memory as it sees fit. If
    /// the value is `Some`, however, then the implementation is expected to
    /// reserve that many bytes for the memory's allocation, plus the guard
    /// size at the end. Note that this reservation need only be a virtual
    /// memory reservation, physical memory does not need to be allocated
    /// immediately. In this case `grow` should never move the base pointer and
    /// the maximum size of `ty` is guaranteed to fit within `reserved_size_in_bytes`.
    ///
    /// The `guard_size_in_bytes` parameter indicates how many bytes of space, after the
    /// memory allocation, is expected to be unmapped. JIT code will elide
    /// bounds checks based on the `guard_size_in_bytes` provided, so for JIT code to
    /// work correctly the memory returned will need to be properly guarded with
    /// `guard_size_in_bytes` bytes left unmapped after the base allocation.
    ///
    /// Note that the `reserved_size_in_bytes` and `guard_size_in_bytes` options are tuned from
    /// the various [`Config`](crate::Config) methods about memory
    /// sizes/guards. Additionally these two values are guaranteed to be
    /// multiples of the system page size.
    fn new_memory(
        &self,
        ty: MemoryType,
        reserved_size_in_bytes: Option<u64>,
        guard_size_in_bytes: u64,
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
        let store = Store::new(&Engine::new(&cfg));
        let ty = MemoryType::new(Limits::new(1, None));
        let mem = Memory::new(&store, ty);
        assert_eq!(mem.wasmtime_export.memory.offset_guard_size, 0);
        match mem.wasmtime_export.memory.style {
            wasmtime_environ::MemoryStyle::Dynamic => {}
            other => panic!("unexpected style {:?}", other),
        }
    }
}
