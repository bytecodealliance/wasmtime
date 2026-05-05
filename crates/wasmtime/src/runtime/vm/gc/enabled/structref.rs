use crate::{
    StorageType, Val,
    prelude::*,
    runtime::vm::{GcHeap, GcStore, VMGcRef},
    store::AutoAssertNoGc,
};
use core::fmt;
use wasmtime_environ::{GcStructLayout, VMGcKind};

/// A `VMGcRef` that we know points to a `struct`.
///
/// Create a `VMStructRef` via `VMGcRef::into_structref` and
/// `VMGcRef::as_structref`, or their untyped equivalents
/// `VMGcRef::into_structref_unchecked` and `VMGcRef::as_structref_unchecked`.
///
/// Note: This is not a `TypedGcRef<_>` because each collector can have a
/// different concrete representation of `structref` that they allocate inside
/// their heaps.
#[derive(Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VMStructRef(VMGcRef);

impl fmt::Pointer for VMStructRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.0, f)
    }
}

impl From<VMStructRef> for VMGcRef {
    #[inline]
    fn from(x: VMStructRef) -> Self {
        x.0
    }
}

impl VMGcRef {
    /// Is this `VMGcRef` pointing to a `struct`?
    pub fn is_structref(&self, gc_heap: &(impl GcHeap + ?Sized)) -> bool {
        if self.is_i31() {
            return false;
        }

        match gc_heap.header(&self) {
            Ok(header) => header.kind().matches(VMGcKind::StructRef),
            Err(_) => false,
        }
    }

    /// Create a new `VMStructRef` from the given `gc_ref`.
    ///
    /// If this is not a GC reference to an `structref`, `Err(self)` is
    /// returned.
    pub fn into_structref(self, gc_heap: &impl GcHeap) -> Result<VMStructRef, VMGcRef> {
        if self.is_structref(gc_heap) {
            Ok(self.into_structref_unchecked())
        } else {
            Err(self)
        }
    }

    /// Create a new `VMStructRef` from `self` without actually checking that
    /// `self` is an `structref`.
    ///
    /// This method does not check that `self` is actually an `structref`, but
    /// it should be. Failure to uphold this invariant is memory safe but will
    /// result in general incorrectness down the line such as panics or wrong
    /// results.
    #[inline]
    pub fn into_structref_unchecked(self) -> VMStructRef {
        debug_assert!(!self.is_i31());
        VMStructRef(self)
    }

    /// Get this GC reference as an `structref` reference, if it actually is an
    /// `structref` reference.
    pub fn as_structref(&self, gc_heap: &(impl GcHeap + ?Sized)) -> Option<&VMStructRef> {
        if self.is_structref(gc_heap) {
            Some(self.as_structref_unchecked())
        } else {
            None
        }
    }

    /// Get this GC reference as an `structref` reference without checking if it
    /// actually is an `structref` reference.
    ///
    /// Calling this method on a non-`structref` reference is memory safe, but
    /// will lead to general incorrectness like panics and wrong results.
    pub fn as_structref_unchecked(&self) -> &VMStructRef {
        debug_assert!(!self.is_i31());
        let ptr = self as *const VMGcRef;
        let ret = unsafe { &*ptr.cast() };
        assert!(matches!(ret, VMStructRef(VMGcRef { .. })));
        ret
    }
}

impl VMStructRef {
    /// Get the underlying `VMGcRef`.
    pub fn as_gc_ref(&self) -> &VMGcRef {
        &self.0
    }

    /// Clone this `VMStructRef`, running any GC barriers as necessary.
    pub fn clone(&self, gc_store: &mut GcStore) -> Self {
        Self(gc_store.clone_gc_ref(&self.0))
    }

    /// Explicitly drop this `structref`, running GC drop barriers as necessary.
    pub fn drop(self, gc_store: &mut GcStore) {
        gc_store.drop_gc_ref(self.0);
    }

    /// Copy this `VMStructRef` without running the GC's clone barriers.
    ///
    /// Prefer calling `clone(&mut GcStore)` instead! This is mostly an internal
    /// escape hatch for collector implementations.
    ///
    /// Failure to run GC barriers when they would otherwise be necessary can
    /// lead to leaks, panics, and wrong results. It cannot lead to memory
    /// unsafety, however.
    pub fn unchecked_copy(&self) -> Self {
        Self(self.0.unchecked_copy())
    }

    /// Read a field of the given `StorageType` into a `Val`.
    ///
    /// `i8` and `i16` fields are zero-extended into `Val::I32(_)`s.
    ///
    /// Does not check that the field is actually of type `ty`. That is the
    /// caller's responsibility. Failure to do so is memory safe, but will lead
    /// to general incorrectness such as panics and wrong results.
    ///
    /// Panics on out-of-bounds accesses.
    pub fn read_field(
        &self,
        store: &mut AutoAssertNoGc,
        layout: &GcStructLayout,
        ty: &StorageType,
        field: usize,
    ) -> Result<Val> {
        let offset = layout.fields[field].offset;
        self.as_gc_ref().read_val(store, ty, offset)
    }

    /// Write the given value into this struct at the given offset.
    ///
    /// Returns an error if `val` is a GC reference that has since been
    /// unrooted.
    ///
    /// Does not check that `val` matches `ty`, nor that the field is actually
    /// of type `ty`. Checking those things is the caller's responsibility.
    /// Failure to do so is memory safe, but will lead to general incorrectness
    /// such as panics and wrong results.
    ///
    /// Panics on out-of-bounds accesses.
    pub fn write_field(
        &self,
        store: &mut AutoAssertNoGc,
        layout: &GcStructLayout,
        ty: &StorageType,
        field: usize,
        val: Val,
    ) -> Result<()> {
        debug_assert!(val._matches_ty(&store, &ty.unpack())?);

        let offset = layout.fields[field].offset;
        self.as_gc_ref().write_val(store, ty, offset, val)
    }

    /// Initialize a field in this structref that is currently uninitialized.
    ///
    /// The difference between this method and `write_field` is that GC barriers
    /// are handled differently. When overwriting an initialized field (aka
    /// `write_field`) we need to call the full write GC write barrier, which
    /// logically drops the old GC reference and clones the new GC
    /// reference. When we are initializing a field for the first time, there is
    /// no old GC reference that is being overwritten and which we need to drop,
    /// so we only need to clone the new GC reference.
    ///
    /// Calling this method on a structref that has already had the associated
    /// field initialized will result in GC bugs. These are memory safe but will
    /// lead to generally incorrect behavior such as panics, leaks, and
    /// incorrect results.
    ///
    /// Does not check that `val` matches `ty`, nor that the field is actually
    /// of type `ty`. Checking those things is the caller's responsibility.
    /// Failure to do so is memory safe, but will lead to general incorrectness
    /// such as panics and wrong results.
    ///
    /// Returns an error if `val` is a GC reference that has since been
    /// unrooted.
    ///
    /// Panics on out-of-bounds accesses.
    pub fn initialize_field(
        &self,
        store: &mut AutoAssertNoGc,
        layout: &GcStructLayout,
        ty: &StorageType,
        field: usize,
        val: Val,
    ) -> Result<()> {
        debug_assert!(val._matches_ty(&store, &ty.unpack())?);
        let offset = layout.fields[field].offset;
        self.as_gc_ref().initialize_val(store, ty, offset, val)
    }
}
