use crate::error::{Error, OomOrDynError};
use core::{fmt, mem, ptr::NonNull};

/// An out-of-memory (OOM) error.
///
/// This error is the sentinel for allocation failure due to memory exhaustion.
///
/// Constructing an [`Error`] from an `OutOfMemory` does not allocate.
///
/// Allocation failure inside any `Error` method that must allocate
/// (e.g. [`Error::context`]) will propagate an `OutOfMemory` error.
///
/// # Out-of-Memory Handling in Wasmtime
///
/// Wasmtime performs out-of-memory (OOM) error handling on a **best-effort
/// basis**. OOM handling does not have [tier 1
/// support](https://docs.wasmtime.dev/stability-tiers.html) at this time and,
/// therefore, while failure to handle OOM at some allocation site may be
/// considered a bug, and might be a potential denial-of-service vector, it
/// would not be considered a security vulnerability.[^limits]
///
/// [^limits]: Note that unconstrained guest-controlled resource usage is still
/// considered a vulnerability. Wasmtime has tier 1 support for limiting guest
/// resources, but not for handling OOMs within those limits.
///
/// ## Where Wasmtime Attempts to Handle OOM
///
/// It is important to note that **not all portions of Wasmtime attempt to
/// handle out-of-memory errors**. Notably, Wasmtime only ever attempts to
/// handle OOM in the core *runtime* and never in the *compiler*. No attempt is
/// made to handle allocation failure in the middle of compiling new `Module`s
/// or `Component`s from Wasm to machine code (or Pulley bytecode). However,
/// Wasmtime will attempt to handle OOM when running *pre-compiled* Wasm code
/// (loaded via `Module::deserialize` or `Component::deserialize`).
///
/// Wasmtime's interfaces allow *you* to handle OOM in your own embedding's WASI
/// implementations and host APIs, but Wasmtime's provided WASI implementations
/// (e.g. `wasmtime_wasi_http`) will generally not attempt to handle OOM (as
/// they often depend on third-party crates that do not attempt to handle OOM).
///
/// The API documentation for individual functions and methods that handle OOM
/// should generally document this fact by listing `OutOfMemory` as one of the
/// potential errors returned.
///
/// | **Where**                                       | **Handles OOM?**                |
/// |-------------------------------------------------|---------------------------------|
/// | **Compiler**                                    | **No**                          |
/// | &emsp;`wasmtime::Module::new`                   | No                              |
/// | &emsp;`wasmtime::Component::new`                | No                              |
/// | &emsp;`wasmtime::CodeBuilder`                   | No                              |
/// | &emsp;Other compilation APIs...                 | No                              |
/// | **Runtime**                                     | **Yes**                         |
/// | &emsp;`wasmtime::Store`                         | Yes                             |
/// | &emsp;`wasmtime::Linker`                        | Yes                             |
/// | &emsp;`wasmtime::Module::deserialize`           | Yes                             |
/// | &emsp;`wasmtime::Instance`                      | Yes                             |
/// | &emsp;`wasmtime::Func::call`                    | Yes                             |
/// | &emsp;Component Model concurrency/async APIs    | Not yet                         |
/// | &emsp;Other instantiation and execution APIs... | Yes                             |
/// | **WASI Implementations and Host APIs**          | **Depends**                     |
/// | &emsp;`wasmtime_wasi`                           | No                              |
/// | &emsp;`wasmtime_wasi_http`                      | No                              |
/// | &emsp;`wasmtime_wasi_*`                         | No                              |
/// | &emsp;Your embedding's APIs                     | If *you* implement OOM handling |
///
/// If you encounter an unhandled OOM inside Wasmtime, and it is within a
/// portion of code where it should be handled, then please [file an
/// issue](https://github.com/bytecodealliance/wasmtime/issues/new/choose).
///
/// ## Handling More OOMs with Rust Nightly APIs
///
/// Rust's standard library provides fallible allocation APIs, or the necessary
/// building blocks for making our own fallible allocation APIs, for some of its
/// types and collections. For example, it provides `Vec::try_reserve` which can
/// be used to build a fallible version of `Vec::push` and fallible `Box`
/// allocation can be built upon raw allocations from the global allocator and
/// `Box::from_raw`.
///
/// However, the standard library does not provide these things for all the
/// types and collections that Wasmtime uses. Some of these APIs are completely
/// missing (such as a fallible version of
/// `std::collections::hash_map::VacantEntry::insert`) and some APIs exist but
/// are feature-gated on unstable, nightly-only Rust features. The most relevant
/// API from this latter category is
/// [`Arc::try_new`](https://doc.rust-lang.org/nightly/std/sync/struct.Arc.html#method.try_new),
/// as Wasmtime's runtime uses a number of `Arc`s under the covers.
///
/// If handling OOMs is important for your Wasmtime embedding, then you should
/// compile Wasmtime from source using a Nightly Rust toolchain and with the
/// `RUSTFLAGS="--cfg arc_try_new"` environment variable set. This unlocks
/// Wasmtime's internal usage of `Arc::try_new`, making more OOM handling at
/// more allocation sites possible.
#[derive(Clone, Copy)]
// NB: `OutOfMemory`'s representation must be the same as `OomOrDynError`
// (and therefore also `Error`).
#[repr(transparent)]
pub struct OutOfMemory {
    inner: NonNull<u8>,
}

// Safety: The `inner` pointer is not a real pointer, it is just bitpacked size
// data.
unsafe impl Send for OutOfMemory {}

// Safety: The `inner` pointer is not a real pointer, it is just bitpacked size
// data.
unsafe impl Sync for OutOfMemory {}

impl fmt::Debug for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutOfMemory")
            .field(
                "requested_allocation_size",
                &self.requested_allocation_size(),
            )
            .finish()
    }
}

impl fmt::Display for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "out of memory (failed to allocate {} bytes)",
            self.requested_allocation_size()
        )
    }
}

impl core::error::Error for OutOfMemory {
    #[inline]
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }
}

impl OutOfMemory {
    // NB: `OutOfMemory`'s representation must be the same as `OomOrDynError`
    // (and therefore also `Error`).
    const _SAME_SIZE_AS_OOM_OR_DYN_ERROR: () =
        assert!(mem::size_of::<OutOfMemory>() == mem::size_of::<OomOrDynError>());
    const _SAME_ALIGN_AS_OOM_OR_DYN_ERROR: () =
        assert!(mem::align_of::<OutOfMemory>() == mem::align_of::<OomOrDynError>());
    const _SAME_SIZE_AS_ERROR: () =
        assert!(mem::size_of::<OutOfMemory>() == mem::size_of::<Error>());
    const _SAME_ALIGN_AS_ERROR: () =
        assert!(mem::align_of::<OutOfMemory>() == mem::align_of::<Error>());

    /// Construct a new `OutOfMemory` error.
    ///
    /// The `requested_allocation_size` argument should be the size (in bytes)
    /// of the associated allocation that was attempted and failed.
    ///
    /// This operation does not allocate.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use wasmtime_internal_core::error::OutOfMemory;
    /// # extern crate alloc;
    /// use alloc::alloc::{Layout, alloc};
    /// use core::ptr::NonNull;
    ///
    /// /// Attempt to allocate a block of memory from the global allocator,
    /// /// returning an `OutOfMemory` error on failure.
    /// fn try_global_alloc(layout: Layout) -> Result<NonNull<u8>, OutOfMemory> {
    ///     if layout.size() == 0 {
    ///         return Ok(NonNull::dangling());
    ///     }
    ///
    ///     // Safety: the layout's size is non-zero.
    ///     let ptr = unsafe { alloc(layout) };
    ///
    ///     if let Some(ptr) = NonNull::new(ptr) {
    ///         Ok(ptr)
    ///     } else {
    ///         // The allocation failed, so return an `OutOfMemory` error,
    ///         // passing the attempted allocation's size into the `OutOfMemory`
    ///         // constructor.
    ///         Err(OutOfMemory::new(layout.size()))
    ///     }
    /// }
    /// ```
    #[inline]
    pub const fn new(requested_allocation_size: usize) -> Self {
        Self {
            inner: OomOrDynError::new_oom_ptr(requested_allocation_size),
        }
    }

    /// Get the size (in bytes) of the associated allocation that was attempted
    /// and which failed.
    ///
    /// Very large allocation sizes (near `isize::MAX` and larger) may be capped
    /// to a maximum value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use wasmtime_internal_core::error::OutOfMemory;
    /// let oom = OutOfMemory::new(8192);
    /// assert_eq!(oom.requested_allocation_size(), 8192);
    /// ```
    #[inline]
    pub fn requested_allocation_size(&self) -> usize {
        OomOrDynError::oom_size(self.inner)
    }
}

impl From<OutOfMemory> for OomOrDynError {
    fn from(oom: OutOfMemory) -> Self {
        OomOrDynError::new_oom(oom.inner)
    }
}
