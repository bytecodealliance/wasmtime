//! Implementation of `exnref` in Wasmtime.

use crate::runtime::vm::VMGcRef;
use crate::store::{Asyncness, StoreResourceLimiter};
use crate::type_registry::RegisteredType;
#[cfg(feature = "async")]
use crate::vm::VMStore;
use crate::vm::{self, VMExnRef, VMGcHeader};
use crate::{
    AsContextMut, GcHeapOutOfMemory, GcRefImpl, GcRootIndex, HeapType, OwnedRooted, RefType,
    Rooted, StorageType, StoreContextMut, Tag, Val, ValRaw, ValType, WasmTy,
    prelude::*,
    store::{AutoAssertNoGc, StoreOpaque},
};
use alloc::sync::Arc;
use core::mem;
use core::mem::MaybeUninit;
use wasmtime_environ::{GcLayout, GcStructLayout, VMGcKind, VMSharedTypeIndex};

/// An allocator for exception objects thrown with a particular tag.
///
/// Every `ExnRefPre` is associated with a particular [`Store`](crate::Store)
/// and a particular [`Tag`] within that store. The tag determines both the
/// exception object's payload types and the tag that the resulting exception
/// objects are thrown with.
///
/// Reusing an allocator across many allocations amortizes some
/// per-type runtime overheads inside Wasmtime. An `ExnRefPre` is to
/// `ExnRef`s as an `InstancePre` is to `Instance`s.
///
/// # Example
///
/// ```
/// use wasmtime::*;
///
/// # fn foo() -> Result<()> {
/// let engine = Engine::default();
/// let mut store = Store::new(&engine, ());
///
/// // Create a tag whose exception objects carry a single `i32` payload.
/// let tag_ty = TagType::new(FuncType::new(&engine, [ValType::I32], []));
/// let tag = Tag::new(&mut store, &tag_ty)?;
///
/// // Create an allocator for exception objects thrown with that tag.
/// let allocator = ExnRefPre::new(&mut store, tag)?;
///
/// {
///     let mut scope = RootScope::new(&mut store);
///
///     // Allocate a bunch of exception objects using the same allocator! This
///     // is faster than creating a new allocator for each object we want to
///     // allocate.
///     for i in 0..10 {
///         ExnRef::new(&mut scope, &allocator, &[Val::I32(i)])?;
///     }
/// }
/// # Ok(())
/// # }
/// # foo().unwrap();
/// ```
pub struct ExnRefPre {
    tag: Tag,

    /// The types of this tag's payload values, i.e. the exception object's
    /// embedder-managed fields.
    field_tys: Box<[StorageType]>,

    /// The synthetic struct type describing the layout of exception objects
    /// thrown with `tag`.
    ty: RegisteredType,
}

impl ExnRefPre {
    /// Create a new `ExnRefPre` that allocates exception objects for the given
    /// tag.
    ///
    /// # Errors
    ///
    /// Returns an error if the store's engine does not have a GC runtime
    /// enabled, or if the tag's type has results (exception payloads are
    /// described by a tag's parameters, so its results must be empty).
    ///
    /// # Panics
    ///
    /// Panics if the tag is not associated with the given store.
    pub fn new(mut store: impl AsContextMut, tag: Tag) -> Result<Self> {
        Self::_new(store.as_context_mut().0, tag)
    }

    pub(crate) fn _new(store: &mut StoreOpaque, tag: Tag) -> Result<Self> {
        assert!(
            tag.comes_from_same_store(store),
            "tag comes from the wrong store"
        );
        ensure!(
            store.engine().gc_runtime().is_some(),
            "cannot allocate exception objects without a GC runtime enabled"
        );

        let tag_ty = tag._ty(store);
        let func_ty = tag_ty.ty();
        ensure!(
            func_ty.results().len() == 0,
            "cannot allocate exception objects for a tag whose type has results"
        );

        let field_tys = func_ty
            .params()
            .map(|ty| {
                assert!(ty.comes_from_same_engine(store.engine()));
                StorageType::ValType(ty)
            })
            .try_collect::<Box<[_]>, _>()?;

        let ty = wasmtime_environ::exn_layout_type(
            field_tys.iter().map(|ty| ty.unpack().to_wasm_type()),
        )?;
        let ty = RegisteredType::new(store.engine(), ty)?;

        store.insert_gc_host_alloc_type(ty.clone());
        Ok(ExnRefPre { tag, field_tys, ty })
    }
}

/// An `exnref` GC reference.
///
/// The `ExnRef` type represents WebAssembly `exnref` values. These
/// are references to exception objects created either by catching a
/// thrown exception in WebAssembly with a `catch_ref` clause of a
/// `try_table`, or by allocating via the host API.
///
/// Note that you can also use `Rooted<ExnRef>` and `OwnedRooted<ExnRef>` as
/// a type parameter with [`Func::typed`][crate::Func::typed]- and
/// [`Func::wrap`][crate::Func::wrap]-style APIs.
#[derive(Debug)]
#[repr(transparent)]
pub struct ExnRef {
    pub(super) inner: GcRootIndex,
}

unsafe impl GcRefImpl for ExnRef {
    fn transmute_ref(index: &GcRootIndex) -> &Self {
        // Safety: `ExnRef` is a newtype of a `GcRootIndex`.
        let me: &Self = unsafe { mem::transmute(index) };

        // Assert we really are just a newtype of a `GcRootIndex`.
        assert!(matches!(
            me,
            Self {
                inner: GcRootIndex { .. },
            }
        ));

        me
    }
}

impl ExnRef {
    /// Creates a new strongly-owned [`ExnRef`] from the raw value provided.
    ///
    /// This is intended to be used in conjunction with [`Func::new_unchecked`],
    /// [`Func::call_unchecked`], and [`ValRaw`] with its `anyref` field.
    ///
    /// This function assumes that `raw` is an `exnref` value which is currently
    /// rooted within the [`Store`].
    ///
    /// # Correctness
    ///
    /// This function is tricky to get right because `raw` not only must be a
    /// valid `exnref` value produced prior by [`ExnRef::to_raw`] but it must
    /// also be correctly rooted within the store. When arguments are provided
    /// to a callback with [`Func::new_unchecked`], for example, or returned via
    /// [`Func::call_unchecked`], if a GC is performed within the store then
    /// floating `exnref` values are not rooted and will be GC'd, meaning that
    /// this function will no longer be correct to call with the values cleaned
    /// up. This function must be invoked *before* possible GC operations can
    /// happen (such as calling Wasm).
    ///
    /// When in doubt try to not use this. Instead use the Rust APIs of
    /// [`TypedFunc`] and friends. Note though that this function is not
    /// `unsafe` as any value can be passed in. Incorrect values can result in
    /// runtime panics, however, so care must still be taken with this method.
    ///
    /// [`Func::call_unchecked`]: crate::Func::call_unchecked
    /// [`Func::new_unchecked`]: crate::Func::new_unchecked
    /// [`Store`]: crate::Store
    /// [`TypedFunc`]: crate::TypedFunc
    /// [`ValRaw`]: crate::ValRaw
    pub fn from_raw(mut store: impl AsContextMut, raw: u32) -> Option<Rooted<Self>> {
        let mut store = AutoAssertNoGc::new(store.as_context_mut().0);
        Self::_from_raw(&mut store, raw)
    }

    // (Not actually memory unsafe since we have indexed GC heaps.)
    pub(crate) fn _from_raw(store: &mut AutoAssertNoGc, raw: u32) -> Option<Rooted<Self>> {
        let gc_ref = VMGcRef::from_raw_u32(raw)?;
        let gc_ref = store.clone_gc_ref(&gc_ref);
        Some(Self::from_cloned_gc_ref(store, gc_ref))
    }

    /// Synchronously allocate a new exception object and get a
    /// reference to it.
    ///
    /// # Automatic Garbage Collection
    ///
    /// If the GC heap is at capacity, and there isn't room for
    /// allocating this new exception object, then this method will
    /// automatically trigger a synchronous collection in an attempt
    /// to free up space in the GC heap.
    ///
    /// # Errors
    ///
    /// If the given `fields` values' types do not match the field
    /// types of the `allocator`'s exception type, an error is
    /// returned.
    ///
    /// If the allocation cannot be satisfied because the GC heap is currently
    /// out of memory, then a [`GcHeapOutOfMemory<()>`][crate::GcHeapOutOfMemory]
    /// error is returned. The allocation might succeed on a second attempt if
    /// you drop some rooted GC references and try again.
    ///
    /// If `store` is configured with a
    /// [`ResourceLimiterAsync`](crate::ResourceLimiterAsync) then an error
    /// will be returned because [`ExnRef::new_async`] should be used instead.
    ///
    /// # Panics
    ///
    /// Panics if the allocator, or any of the field values, is not associated
    /// with the given store.
    pub fn new(
        mut store: impl AsContextMut,
        allocator: &ExnRefPre,
        fields: &[Val],
    ) -> Result<Rooted<ExnRef>> {
        let (mut limiter, store) = store
            .as_context_mut()
            .0
            .validate_sync_resource_limiter_and_store_opaque()?;
        vm::assert_ready(Self::_new_async(
            store,
            limiter.as_mut(),
            allocator,
            fields,
            Asyncness::No,
        ))
    }

    /// Asynchronously allocate a new exception object and get a
    /// reference to it.
    ///
    /// # Automatic Garbage Collection
    ///
    /// If the GC heap is at capacity, and there isn't room for allocating this
    /// new exn, then this method will automatically trigger a synchronous
    /// collection in an attempt to free up space in the GC heap.
    ///
    /// # Errors
    ///
    /// If the given `fields` values' types do not match the field
    /// types of the `allocator`'s exception type, an error is
    /// returned.
    ///
    /// If the allocation cannot be satisfied because the GC heap is currently
    /// out of memory, then a [`GcHeapOutOfMemory<()>`][crate::GcHeapOutOfMemory]
    /// error is returned. The allocation might succeed on a second attempt if
    /// you drop some rooted GC references and try again.
    ///
    /// # Panics
    ///
    /// Panics if the allocator, or any of the field values, is not associated
    /// with the given store.
    #[cfg(feature = "async")]
    pub async fn new_async(
        mut store: impl AsContextMut,
        allocator: &ExnRefPre,
        fields: &[Val],
    ) -> Result<Rooted<ExnRef>> {
        let (mut limiter, store) = store.as_context_mut().0.resource_limiter_and_store_opaque();
        Self::_new_async(store, limiter.as_mut(), allocator, fields, Asyncness::Yes).await
    }

    pub(crate) async fn _new_async(
        store: &mut StoreOpaque,
        limiter: Option<&mut StoreResourceLimiter<'_>>,
        allocator: &ExnRefPre,
        fields: &[Val],
        asyncness: Asyncness,
    ) -> Result<Rooted<ExnRef>> {
        ensure!(
            allocator.tag.comes_from_same_store(store),
            "attempted to use an `ExnRefPre` with the wrong store",
        );
        Self::type_check_fields(store, allocator, fields)?;
        store
            .retry_after_gc_async(limiter, (), asyncness, |store, ()| {
                Self::new_unchecked(store, allocator, fields)
            })
            .await
    }

    /// Type check the field values before allocating a new exception object.
    fn type_check_fields(
        store: &mut StoreOpaque,
        allocator: &ExnRefPre,
        fields: &[Val],
    ) -> Result<(), Error> {
        assert!(allocator.tag.comes_from_same_store(store));
        let expected_len = allocator.field_tys.len();
        let actual_len = fields.len();
        ensure!(
            actual_len == expected_len,
            "expected {expected_len} fields, got {actual_len}"
        );
        for (ty, val) in allocator.field_tys.iter().zip(fields) {
            assert!(
                val.comes_from_same_store(store),
                "field value comes from the wrong store",
            );
            val.ensure_matches_ty(store, ty.unpack())
                .context("field type mismatch")?;
        }
        Ok(())
    }

    /// Given that the field values have already been type checked, allocate a
    /// new exn.
    ///
    /// Does not attempt GC+retry on OOM, that is the caller's responsibility.
    fn new_unchecked(
        store: &mut StoreOpaque,
        allocator: &ExnRefPre,
        fields: &[Val],
    ) -> Result<Rooted<ExnRef>> {
        // Allocate the exn and write each field value into the appropriate
        // offset.
        let layout = allocator
            .ty
            .layout()
            .expect("exn layout types have a layout")
            .unwrap_struct();
        let exnref = store
            .require_gc_store_mut()?
            .alloc_uninit_exn(allocator.ty.index(), &layout)
            .context("unrecoverable error when allocating new `exnref`")?
            .map_err(|n| GcHeapOutOfMemory::new((), n))?;

        // From this point on, if we get any errors, then the exn is not
        // fully initialized, so we need to eagerly deallocate it before the
        // next GC where the collector might try to interpret one of the
        // uninitialized fields as a GC reference.
        let mut store = AutoAssertNoGc::new(store);
        match (|| {
            let (instance, index) = allocator.tag.to_raw_indices();
            exnref.initialize_tag(&mut store, &layout, instance, index)?;
            for (index, (ty, val)) in allocator.field_tys.iter().zip(fields).enumerate() {
                exnref.initialize_field(&mut store, &layout, ty, index, *val)?;
            }
            Ok(())
        })() {
            Ok(()) => Ok(Rooted::new(&mut store, exnref.into())),
            Err(e) => {
                store.require_gc_store_mut()?.dealloc_uninit_exn(exnref)?;
                Err(e)
            }
        }
    }

    pub(crate) fn type_index(&self, store: &StoreOpaque) -> Result<VMSharedTypeIndex> {
        let gc_ref = self.inner.try_gc_ref(store)?;
        let header = store.require_gc_store()?.header(gc_ref)?;
        debug_assert!(header.kind().matches(VMGcKind::ExnRef));
        Ok(header.ty().expect("exnrefs should have concrete types"))
    }

    /// Create a new `Rooted<ExnRef>` from the given GC reference.
    ///
    /// `gc_ref` should point to a valid `exnref` and should belong to
    /// the store's GC heap. Failure to uphold these invariants is
    /// memory safe but will lead to general incorrectness such as
    /// panics or wrong results.
    pub(crate) fn from_cloned_gc_ref(
        store: &mut AutoAssertNoGc<'_>,
        gc_ref: VMGcRef,
    ) -> Rooted<Self> {
        debug_assert!(
            store
                .unwrap_gc_store()
                .kind(&gc_ref)
                .unwrap()
                .matches(VMGcKind::ExnRef)
        );
        Rooted::new(store, gc_ref)
    }

    #[inline]
    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        self.inner.comes_from_same_store(store)
    }

    /// Converts this [`ExnRef`] to a raw value suitable to store within a
    /// [`ValRaw`].
    ///
    /// Returns an error if this `exnref` has been unrooted.
    ///
    /// # Correctness
    ///
    /// Produces a raw value which is only valid to pass into a store if a GC
    /// doesn't happen between when the value is produce and when it's passed
    /// into the store.
    ///
    /// [`ValRaw`]: crate::ValRaw
    pub fn to_raw(&self, mut store: impl AsContextMut) -> Result<u32> {
        let mut store = AutoAssertNoGc::new(store.as_context_mut().0);
        self._to_raw(&mut store)
    }

    pub(crate) fn _to_raw(&self, store: &mut AutoAssertNoGc<'_>) -> Result<u32> {
        self.inner.expose_gc_ref_to_wasm(store).map(|r| r.get())
    }

    /// Get the values of this exception object's fields.
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn fields<'a, T: 'static>(
        &'a self,
        store: impl Into<StoreContextMut<'a, T>>,
    ) -> Result<impl ExactSizeIterator<Item = Val> + 'a> {
        self._fields(store.into().0)
    }

    pub(crate) fn _fields<'a>(
        &'a self,
        store: &'a mut StoreOpaque,
    ) -> Result<impl ExactSizeIterator<Item = Val> + 'a> {
        assert!(self.comes_from_same_store(store));
        let len = self.tag_(store)?._ty(store).ty().params().len();
        let store = AutoAssertNoGc::new(store);

        let gc_ref = self.inner.try_gc_ref(&store)?;
        let header = store.require_gc_store()?.header(gc_ref)?;
        debug_assert!(header.kind().matches(VMGcKind::ExnRef));

        return Ok(Fields {
            exnref: self,
            store,
            index: 0,
            len,
        });

        struct Fields<'a, 'b> {
            exnref: &'a ExnRef,
            store: AutoAssertNoGc<'b>,
            index: usize,
            len: usize,
        }

        impl Iterator for Fields<'_, '_> {
            type Item = Val;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                let i = self.index;
                debug_assert!(i <= self.len);
                if i >= self.len {
                    return None;
                }
                self.index += 1;
                self.exnref._field(&mut self.store, i).ok()
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                let len = self.len - self.index;
                (len, Some(len))
            }
        }

        impl ExactSizeIterator for Fields<'_, '_> {
            #[inline]
            fn len(&self) -> usize {
                self.len - self.index
            }
        }
    }

    fn header<'a>(&self, store: &'a StoreOpaque) -> Result<&'a VMGcHeader> {
        assert!(self.comes_from_same_store(store));
        let gc_ref = self.inner.try_gc_ref(store)?;
        Ok(store.require_gc_store()?.header(gc_ref)?)
    }

    fn exnref<'a>(&self, store: &'a StoreOpaque) -> Result<&'a VMExnRef> {
        assert!(self.comes_from_same_store(store));
        let gc_ref = self.inner.try_gc_ref(store)?;
        debug_assert!(self.header(store)?.kind().matches(VMGcKind::ExnRef));
        Ok(gc_ref.as_exnref_unchecked())
    }

    fn layout(&self, store: &StoreOpaque) -> Result<Arc<GcStructLayout>> {
        assert!(self.comes_from_same_store(store));
        let type_index = self.type_index(store)?;
        let layout = store
            .engine()
            .signatures()
            .layout(type_index)
            .expect("exn types should have GC layouts");
        match layout {
            GcLayout::Struct(s) => Ok(s),
            GcLayout::Array(_) => unreachable!(),
        }
    }

    fn field_ty(&self, store: &StoreOpaque, field: usize) -> Result<ValType> {
        let tag_ty = self.tag_(store)?._ty(store);
        let mut fields = tag_ty.ty().params();
        let len = fields.len();
        match fields.nth(field) {
            Some(f) => Ok(f),
            None => {
                bail!("cannot access field {field}: exn only has {len} fields")
            }
        }
    }

    /// Get this exception object's `index`th field.
    ///
    /// # Errors
    ///
    /// Returns an `Err(_)` if the index is out of bounds or this reference has
    /// been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn field(&self, mut store: impl AsContextMut, index: usize) -> Result<Val> {
        let mut store = AutoAssertNoGc::new(store.as_context_mut().0);
        self._field(&mut store, index)
    }

    pub(crate) fn _field(&self, store: &mut AutoAssertNoGc<'_>, index: usize) -> Result<Val> {
        assert!(self.comes_from_same_store(store));
        let exnref = self.exnref(store)?.unchecked_copy();
        let field_ty = self.field_ty(store, index)?;
        let layout = self.layout(store)?;
        exnref.read_field(store, &layout, &field_ty.into(), index)
    }

    /// Get this exception object's associated tag.
    ///
    /// # Errors
    ///
    /// Returns an `Err(_)` if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn tag(&self, mut store: impl AsContextMut) -> Result<Tag> {
        self.tag_(store.as_context_mut().0)
    }

    fn tag_(&self, store: &StoreOpaque) -> Result<Tag> {
        assert!(self.comes_from_same_store(store));
        let exnref = self.exnref(store)?.unchecked_copy();
        let (instance, index) = exnref.tag(store)?;
        Ok(Tag::from_raw_indices(store, instance, index))
    }
}

unsafe impl WasmTy for Rooted<ExnRef> {
    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Exn))
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.comes_from_same_store(store)
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
        // Wasm cannot name a concrete `exn` type, so this is never reached.
        unreachable!()
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        self.wasm_ty_store(store, ptr, ValRaw::anyref)
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        Self::wasm_ty_load(store, ptr.get_anyref(), ExnRef::from_cloned_gc_ref)
    }
}

unsafe impl WasmTy for Option<Rooted<ExnRef>> {
    #[inline]
    fn valtype() -> ValType {
        ValType::EXNREF
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.map_or(true, |x| x.comes_from_same_store(store))
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
        // Wasm cannot name a concrete `exn` type, so this is never reached.
        unreachable!()
    }

    #[inline]
    fn is_vmgcref_and_points_to_object(&self) -> bool {
        self.is_some()
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        <Rooted<ExnRef>>::wasm_ty_option_store(self, store, ptr, ValRaw::anyref)
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        <Rooted<ExnRef>>::wasm_ty_option_load(store, ptr.get_anyref(), ExnRef::from_cloned_gc_ref)
    }
}

unsafe impl WasmTy for OwnedRooted<ExnRef> {
    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Exn))
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.comes_from_same_store(store)
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
        // Wasm cannot name a concrete `exn` type, so this is never reached.
        unreachable!()
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        self.wasm_ty_store(store, ptr, ValRaw::anyref)
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        Self::wasm_ty_load(store, ptr.get_anyref(), ExnRef::from_cloned_gc_ref)
    }
}

unsafe impl WasmTy for Option<OwnedRooted<ExnRef>> {
    #[inline]
    fn valtype() -> ValType {
        ValType::EXNREF
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.as_ref()
            .map_or(true, |x| x.comes_from_same_store(store))
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
        // Wasm cannot name a concrete `exn` type, so this is never reached.
        unreachable!()
    }

    #[inline]
    fn is_vmgcref_and_points_to_object(&self) -> bool {
        self.is_some()
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        <OwnedRooted<ExnRef>>::wasm_ty_option_store(self, store, ptr, ValRaw::anyref)
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        <OwnedRooted<ExnRef>>::wasm_ty_option_load(
            store,
            ptr.get_anyref(),
            ExnRef::from_cloned_gc_ref,
        )
    }
}
