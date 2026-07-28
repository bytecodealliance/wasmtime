use crate::{
    StorageType, Val,
    prelude::*,
    runtime::vm::{GcHeap, GcStore, VMGcRef},
    store::{AutoAssertNoGc, InstanceId, StoreOpaque},
};
use core::fmt;
use wasmtime_environ::{
    DefinedTagIndex, EXN_ENGINE_FIELDS, EXN_TAG_DEFINED_FIELD, EXN_TAG_INSTANCE_FIELD,
    GcStructLayout, VMGcKind,
};

/// A `VMGcRef` that we know points to an `exn`.
///
/// Create a `VMExnRef` via `VMGcRef::into_exnref` and
/// `VMGcRef::as_exnref`, or their untyped equivalents
/// `VMGcRef::into_exnref_unchecked` and `VMGcRef::as_exnref_unchecked`.
///
/// Note: This is not a `TypedGcRef<_>` because each collector can have a
/// different concrete representation of `exnref` that they allocate inside
/// their heaps.
#[derive(Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VMExnRef(VMGcRef);

impl fmt::Pointer for VMExnRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.0, f)
    }
}

impl From<VMExnRef> for VMGcRef {
    #[inline]
    fn from(x: VMExnRef) -> Self {
        x.0
    }
}

impl VMGcRef {
    /// Is this `VMGcRef` pointing to an `exn`?
    pub fn is_exnref(&self, gc_heap: &(impl GcHeap + ?Sized)) -> bool {
        if self.is_i31() {
            return false;
        }

        match gc_heap.header(&self) {
            Ok(header) => header.kind().matches(VMGcKind::ExnRef),
            Err(_) => false,
        }
    }

    /// Create a new `VMExnRef` from the given `gc_ref`.
    ///
    /// If this is not a GC reference to an `exnref`, `Err(self)` is
    /// returned.
    pub fn into_exnref(self, gc_heap: &(impl GcHeap + ?Sized)) -> Result<VMExnRef, VMGcRef> {
        if self.is_exnref(gc_heap) {
            Ok(self.into_exnref_unchecked())
        } else {
            Err(self)
        }
    }

    /// Create a new `VMExnRef` from `self` without actually checking that
    /// `self` is an `exnref`.
    ///
    /// This method does not check that `self` is actually an `exnref`, but
    /// it should be. Failure to uphold this invariant is memory safe but will
    /// result in general incorrectness down the line such as panics or wrong
    /// results.
    #[inline]
    pub fn into_exnref_unchecked(self) -> VMExnRef {
        debug_assert!(!self.is_i31());
        VMExnRef(self)
    }

    /// Get this GC reference as an `exnref` reference, if it actually is an
    /// `exnref` reference.
    pub fn as_exnref(&self, gc_heap: &(impl GcHeap + ?Sized)) -> Option<&VMExnRef> {
        if self.is_exnref(gc_heap) {
            Some(self.as_exnref_unchecked())
        } else {
            None
        }
    }

    /// Get this GC reference as an `exnref` reference without checking if it
    /// actually is an `exnref` reference.
    ///
    /// Calling this method on a non-`exnref` reference is memory safe, but
    /// will lead to general incorrectness like panics and wrong results.
    pub fn as_exnref_unchecked(&self) -> &VMExnRef {
        debug_assert!(!self.is_i31());
        let ptr = self as *const VMGcRef;
        let ret = unsafe { &*ptr.cast() };
        assert!(matches!(ret, VMExnRef(VMGcRef { .. })));
        ret
    }
}

impl VMExnRef {
    /// Get the underlying `VMGcRef`.
    pub fn as_gc_ref(&self) -> &VMGcRef {
        &self.0
    }

    /// Get a mutable borrow on the underlying `VMGcRef`.
    ///
    /// Requires that the mutation retains the reference's invariants,
    /// namely: not null, and pointing to a valid exnref object. Doing
    /// otherwise is memory safe, but will lead to general
    /// incorrectness.
    pub fn as_gc_ref_mut(&mut self) -> &mut VMGcRef {
        &mut self.0
    }

    /// Clone this `VMExnRef`, running any GC barriers as necessary.
    pub fn clone(&self, gc_store: &mut GcStore) -> Self {
        Self(gc_store.clone_gc_ref(&self.0))
    }

    /// Explicitly drop this `exnref`, running GC drop barriers as necessary.
    pub fn drop(self, gc_store: &mut GcStore) {
        gc_store.drop_gc_ref(self.0);
    }

    /// Copy this `VMExnRef` without running the GC's clone barriers.
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

    /// Read a payload field of the given `StorageType` into a `Val`.
    ///
    /// `field` indexes the exception's payload values, skipping past the
    /// leading engine-managed fields of the layout.
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
        let offset = layout.fields[field + EXN_ENGINE_FIELDS].offset;
        self.as_gc_ref().read_val(store, ty, offset)
    }

    /// Initialize a payload field in this exnref that is currently
    /// uninitialized.
    ///
    /// `field` indexes the exception's payload values, skipping past the
    /// leading engine-managed fields of the layout.
    ///
    /// Calling this method on an exnref that has already had the
    /// associated field initialized will result in GC bugs. These are
    /// memory safe but will lead to generally incorrect behavior such
    /// as panics, leaks, and incorrect results.
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
        let offset = layout.fields[field + EXN_ENGINE_FIELDS].offset;
        self.as_gc_ref().initialize_val(store, ty, offset, val)
    }

    /// Initialize the tag referenced by this exception object.
    ///
    /// The tag lives in the leading engine-managed fields of the exception
    /// object's layout; see [`EXN_TAG_INSTANCE_FIELD`].
    pub fn initialize_tag(
        &self,
        store: &mut AutoAssertNoGc,
        layout: &GcStructLayout,
        instance: InstanceId,
        tag: DefinedTagIndex,
    ) -> Result<()> {
        let instance_offset = layout.fields[EXN_TAG_INSTANCE_FIELD].offset;
        let tag_offset = layout.fields[EXN_TAG_DEFINED_FIELD].offset;
        let store = store.require_gc_store_mut()?;
        store
            .gc_object_data_mut(&self.0)?
            .write_u32(instance_offset, instance.as_u32())?;
        store
            .gc_object_data_mut(&self.0)?
            .write_u32(tag_offset, tag.as_u32())?;
        Ok(())
    }

    /// Get the tag referenced by this exception object.
    pub fn tag(&self, store: &StoreOpaque) -> Result<(InstanceId, DefinedTagIndex)> {
        // FIXME: ideally there'd be a way to read the instance/tag fields of
        // this exception going through the `GcLayout` type somehow. Currently
        // acquiring that for an exception requires taking an rwlock on the
        // engine, however, as well as cloning an `Arc` for its layout. This
        // method is on the path of guests throwing exceptions, however, meaning
        // that going the easy route of through the engine introduces contended
        // locks/atomics. This is generally avoided in Wasmtime at this time to
        // avoid horizontal-scaling bottlenecks. If layouts are exposed through
        // the GC store, however, then that would be better to use and then the
        // `exception_*_offset` methods can be removed entirely here, too.
        let layouts = store.engine().gc_runtime().unwrap().layouts();
        let instance_offset = layouts.exception_tag_instance_offset();
        let tag_offset = layouts.exception_tag_defined_offset();
        let gc_store = store.require_gc_store()?;
        let instance = gc_store
            .gc_object_data(&self.0)?
            .read_u32(instance_offset)?;
        let instance = InstanceId::from_u32(instance);
        let tag = gc_store.gc_object_data(&self.0)?.read_u32(tag_offset)?;
        let tag = DefinedTagIndex::from_u32(tag);
        Ok((instance, tag))
    }
}
