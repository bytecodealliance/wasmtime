use crate::error::OomOrDynError;
use core::{fmt, mem, ptr::NonNull};

/// Out-of-memory error.
///
/// This error is the sentinel for allocation failure due to memory exhaustion.
///
/// Constructing an [`Error`][crate::Error] from an `OutOfMemory` does not
/// allocate.
///
/// Allocation failure inside any `Error` method that must allocate
/// (e.g. [`Error::context`][crate::Error::context]) will propagate an
/// `OutOfMemory` error.
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
        assert!(mem::size_of::<OutOfMemory>() == mem::size_of::<crate::Error>());
    const _SAME_ALIGN_AS_ERROR: () =
        assert!(mem::align_of::<OutOfMemory>() == mem::align_of::<crate::Error>());

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
    /// # use wasmtime_internal_error::OutOfMemory;
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
    /// # use wasmtime_internal_error::OutOfMemory;
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
        OomOrDynError { inner: oom.inner }
    }
}
