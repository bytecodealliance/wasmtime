//! Unrooted references: cheap, weak, read-only references to GC objects.

use crate::runtime::epoch_state::EpochStateHandle;
use crate::runtime::vm::{GcHeap, VMGcHeader, VMGcRef};
use crate::store::{AutoAssertNoGc, StoreOpaque};
use crate::{
    AnyRef, ArrayRef, ArrayType, AsContext, AsContextMut, EqRef, ExnRef, ExnType, ExternRef, GcRef,
    HeapType, I31, OwnedRooted, Result, Rooted, StoreContext, StructRef, StructType, Tag, Val,
    bail_bug, prelude::*,
};
use core::any::Any;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker;
use core::num::NonZeroU32;
use wasmtime_environ::{VMGcKind, VMSharedTypeIndex};

/// A read-only reference to a GC object, valid only until the store's
/// GC heap next changes.
///
/// Unlike [`Rooted`] and [`OwnedRooted`], an `Unrooted` reference
/// does not keep its object alive (hence, "unrooted"). This means
/// that it could become invalid with any action that could mutate the
/// GC heap. If it becomes invalid, APIs will fail cleanly (indicate
/// that it is stale) rather than crashing or panicking.
///
/// In exchange for this "weak" aspect, an `Unrooted` reference is
/// *very cheap*: it does not need to do any rooting bookkeeping, so
/// it effectively acts like a `Copy` pointer into the GC heap, plus a
/// tiny "epoch" stamp that allows us to cheaply check if it has
/// become invalid.
///
/// Because the `Unrooted` reference does not need to update any
/// bookkeeping metadata, it can be created with only read-only access
/// to the `Store`. We also return `Unrooted` references from getters
/// and allocation methods by default: the former allows full
/// read-only heap traversal, and the latter works because *after* the
/// allocation, the heap doesn't mutate. In both cases, the user is
/// free to root the value themselves (see [`Unrooted::to_rooted`] or
/// [`Unrooted::to_owned_rooted`]). This "opt-in" rooting is an
/// intentional pay-as-you-go design choice.
///
/// # Invalidation
///
/// In general, an `Unrooted` reference may (but is not guaranteed
/// to!) become invalid when any method that takes a `&mut Store` is
/// invoked on the corresponding store. This includes both Wasm
/// execution and use of host APIs, for example those that allocate or
/// mutate GC objects.
///
/// # Conversions
///
/// Use [`Rooted::to_unrooted`] or [`OwnedRooted::to_unrooted`] to get an
/// unrooted reference to a rooted object, and [`Unrooted::to_rooted`] or
/// [`Unrooted::to_owned_rooted`] to root the object again, to keep it alive
/// and usable after the heap next changes. Reading a GC reference out of an
/// object through an unrooted reference, for example with
/// [`Unrooted::<StructRef>::field`], gives an unrooted reference as well.
///
/// Since rooting any reference invalidates all unrooted references,
/// it is not possible to root multiple unrooted references
/// one-by-one. Instead, use [`Rooted::to_all_rooted`],
/// [`OwnedRooted::to_all_owned_rooted`], or
/// [`Val::ensure_all_rooted`] to root several unrooted references in
/// a batch.
///
/// Unrooted references compare equal, and hash the same, exactly when they
/// are valid at the same time and refer to the same object (or are equal
/// `i31`s).
pub struct Unrooted<T: GcRef> {
    repr: Repr,
    _phantom: marker::PhantomData<fn() -> T>,
}

/// The representation of an `Unrooted<T>`.
///
/// Note that this is an enum, rather than a struct with an optional heap
/// handle, so that it has a tag with plenty of niches; this keeps `Val` small.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Repr {
    /// An `i31`, which is not an object in the heap, and so is always valid.
    I31(NonZeroU32),

    /// An object in the store's GC heap, at `raw`, valid in the version `heap`
    /// of the heap.
    Object {
        // Note that this has a scary-heavyweight type (`Box`) in it,
        // but the `EpochStateHandle` doesn't actually carry that
        // type! It just allows us to get it from the `EpochState`
        // container (separately passed in) when we want to.
        heap: EpochStateHandle<Box<dyn GcHeap>>,
        raw: NonZeroU32,
    },
}

impl<T: GcRef> Clone for Unrooted<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: GcRef> Copy for Unrooted<T> {}

impl<T: GcRef> fmt::Debug for Unrooted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format!("Unrooted<{}>", core::any::type_name::<T>());
        f.debug_tuple(&name).field(&self.repr).finish()
    }
}

impl<T: GcRef> PartialEq for Unrooted<T> {
    fn eq(&self, other: &Self) -> bool {
        self.repr == other.repr
    }
}

impl<T: GcRef> Eq for Unrooted<T> {}

impl<T: GcRef> Hash for Unrooted<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.repr.hash(state);
    }
}

impl<T: GcRef> Unrooted<T> {
    /// An unrooted reference to `gc_ref`, which must be a `T` in `store`.
    pub(crate) fn new(store: &StoreOpaque, gc_ref: &VMGcRef) -> Self {
        let raw = gc_ref.as_raw_non_zero_u32();
        let repr = if gc_ref.is_i31() {
            Repr::I31(raw)
        } else {
            Repr::Object {
                heap: store.unwrap_gc_store().gc_heap.handle(),
                raw,
            }
        };
        Unrooted {
            repr,
            _phantom: marker::PhantomData,
        }
    }

    /// An unrooted reference to the `T` whose raw bits are `raw`, or `None`
    /// for null.
    pub(crate) fn from_raw_u32(store: &StoreOpaque, raw: u32) -> Option<Self> {
        VMGcRef::from_raw_u32(raw).map(|gc_ref| Self::new(store, &gc_ref))
    }

    /// The referenced object, if this reference is still valid.
    ///
    /// The result borrows `store`, so nothing can collect garbage or otherwise
    /// mutate the GC heap, and hence invalidate this reference, while it is in
    /// use.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub(crate) fn gc_ref<'a>(&'a self, store: &'a StoreOpaque) -> Result<&'a VMGcRef> {
        match &self.repr {
            // `i31`s are not objects in the GC heap, so are always valid.
            Repr::I31(raw) => Ok(VMGcRef::from_frozen_raw_u32(raw)),
            Repr::Object { heap: handle, raw } => {
                let heap = store
                    .optional_gc_store()
                    .map(|s| &s.gc_heap)
                    .filter(|heap| handle.belongs_to(heap));
                let Some(heap) = heap else {
                    panic!("unrooted reference used with the wrong store");
                };
                ensure!(
                    handle.is_valid(heap),
                    "attempted to use an unrooted garbage-collected reference after \
                     the GC heap changed"
                );
                // The object is still live, since `handle` was taken when this
                // reference was created and the heap has not changed since,
                // and it stays so while `store` is borrowed.
                Ok(VMGcRef::from_frozen_raw_u32(raw))
            }
        }
    }

    fn header<'a>(&self, store: &'a StoreOpaque) -> Result<&'a VMGcHeader> {
        let gc_ref = self.gc_ref(store)?;
        store.require_gc_store()?.header(&gc_ref)
    }

    /// The concrete type of the referenced object, which must be of `kind`.
    fn type_index(&self, store: &StoreOpaque, kind: VMGcKind) -> Result<VMSharedTypeIndex> {
        let header = self.header(store)?;
        debug_assert!(header.matches_kind(kind));
        match header.ty() {
            Some(ty) => Ok(ty),
            None => bail_bug!("{kind:?} objects should have concrete types"),
        }
    }

    fn cast<U: GcRef>(self) -> Unrooted<U> {
        Unrooted {
            repr: self.repr,
            _phantom: marker::PhantomData,
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        match &self.repr {
            Repr::I31(_) => true,
            Repr::Object { heap: handle, .. } => store
                .optional_gc_store()
                .is_some_and(|s| handle.belongs_to(&s.gc_heap)),
        }
    }

    /// Is this reference still valid?
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn is_valid(&self, store: impl AsContext) -> bool {
        self.gc_ref(store.as_context().0).is_ok()
    }

    /// Roots the referenced object in the current LIFO rooting scope, to keep
    /// it alive and usable after this reference becomes invalid; see
    /// [`Rooted`].
    ///
    /// Rooting a reference invalidates every other unrooted reference into the
    /// store's GC heap; use [`Rooted::to_all_rooted`] to root several references at once.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn to_rooted(&self, mut store: impl AsContextMut) -> Result<Rooted<T>> {
        self._to_rooted(store.as_context_mut().0)
    }

    pub(crate) fn _to_rooted(&self, store: &mut StoreOpaque) -> Result<Rooted<T>> {
        self.gc_ref(store)?;
        let mut store = AutoAssertNoGc::new(store);
        Ok(self.to_rooted_prevalidated(&mut store))
    }

    /// Roots the referenced object until the returned handle is dropped, to
    /// keep it alive and usable after this reference becomes invalid; see
    /// [`OwnedRooted`].
    ///
    /// Rooting a reference invalidates every other unrooted reference into the
    /// store's GC heap; use [`OwnedRooted::to_all_owned_rooted`] to root several references at once.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// This function will return an [`OutOfMemory`](crate::OutOfMemory)
    /// error when memory allocation fails. See the `OutOfMemory` type's
    /// documentation for details on Wasmtime's out-of-memory handling.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn to_owned_rooted(&self, mut store: impl AsContextMut) -> Result<OwnedRooted<T>> {
        self._to_owned_rooted(store.as_context_mut().0)
    }

    pub(crate) fn _to_owned_rooted(&self, store: &mut StoreOpaque) -> Result<OwnedRooted<T>> {
        self.gc_ref(store)?;
        let mut store = AutoAssertNoGc::new(store);
        self.to_owned_rooted_prevalidated(&mut store)
    }

    /// Check that every one of `refs` is still valid, as the first step of
    /// rooting them all at once; see [`Rooted::to_all_rooted`].
    pub(crate) fn validate_all(store: &StoreOpaque, refs: &[Self]) -> Result<()> {
        for r in refs {
            r.gc_ref(store)?;
        }
        Ok(())
    }

    /// The referenced object, *without* checking that this reference is still
    /// valid.
    ///
    /// Rooting a reference clones it and records a new root, each of which
    /// changes the GC heap, and so invalidates every unrooted reference. To
    /// root several unrooted references at once, we therefore check that they
    /// are all valid first, with `gc_ref` while the heap is unchanged, and then
    /// root each of them with this. That is correct because nothing in between
    /// can free or move an object: we are inside a no-GC scope, and cloning
    /// references and recording roots only ever keep objects alive.
    ///
    /// The caller must have checked, with `gc_ref`, that this reference was
    /// valid, and have done nothing since then but root unrooted references in
    /// this way, all within one `AutoAssertNoGc` scope.
    fn prevalidated_gc_ref(&self) -> VMGcRef {
        match self.repr {
            Repr::I31(raw) | Repr::Object { raw, .. } => VMGcRef::from_raw_non_zero_u32(raw),
        }
    }

    /// Root this reference in the current LIFO rooting scope, after the caller
    /// has checked that it is valid; see `prevalidated_gc_ref` for the
    /// conditions.
    pub(crate) fn to_rooted_prevalidated(&self, store: &mut AutoAssertNoGc<'_>) -> Rooted<T> {
        let gc_ref = store.clone_gc_ref(&self.prevalidated_gc_ref());
        Rooted::new(store, gc_ref)
    }

    /// Root this reference until the returned handle is dropped, after the
    /// caller has checked that it is valid; see `prevalidated_gc_ref` for the
    /// conditions.
    pub(crate) fn to_owned_rooted_prevalidated(
        &self,
        store: &mut AutoAssertNoGc<'_>,
    ) -> Result<OwnedRooted<T>> {
        let gc_ref = store.clone_gc_ref(&self.prevalidated_gc_ref());
        Ok(OwnedRooted::new(store, gc_ref)?)
    }
}

impl Unrooted<AnyRef> {
    /// The type of the referenced object; see [`AnyRef::ty`].
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn ty(&self, store: impl AsContext) -> Result<HeapType> {
        self._ty(store.as_context().0)
    }

    pub(crate) fn _ty(&self, store: &StoreOpaque) -> Result<HeapType> {
        AnyRef::_ty_of(store, self.gc_ref(store)?)
    }

    fn kind(&self, store: &StoreOpaque) -> Result<Option<VMGcKind>> {
        let gc_ref = self.gc_ref(store)?;
        if gc_ref.is_i31() {
            return Ok(None);
        }
        Ok(Some(store.require_gc_store()?.kind(&gc_ref)?))
    }

    /// The referenced `i31`, if this refers to one.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn as_i31(&self, store: impl AsContext) -> Result<Option<I31>> {
        Ok(self.gc_ref(store.as_context().0)?.as_i31().map(Into::into))
    }

    /// This reference as a reference to a struct, if it refers to one.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn as_struct(&self, store: impl AsContext) -> Result<Option<Unrooted<StructRef>>> {
        let kind = self.kind(store.as_context().0)?;
        Ok(kind
            .is_some_and(|k| k.matches(VMGcKind::StructRef))
            .then(|| self.cast()))
    }

    /// This reference as a reference to an array, if it refers to one.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn as_array(&self, store: impl AsContext) -> Result<Option<Unrooted<ArrayRef>>> {
        let kind = self.kind(store.as_context().0)?;
        Ok(kind
            .is_some_and(|k| k.matches(VMGcKind::ArrayRef))
            .then(|| self.cast()))
    }
}

impl From<Unrooted<EqRef>> for Unrooted<AnyRef> {
    fn from(r: Unrooted<EqRef>) -> Self {
        r.cast()
    }
}

impl From<Unrooted<StructRef>> for Unrooted<AnyRef> {
    fn from(r: Unrooted<StructRef>) -> Self {
        r.cast()
    }
}

impl From<Unrooted<ArrayRef>> for Unrooted<AnyRef> {
    fn from(r: Unrooted<ArrayRef>) -> Self {
        r.cast()
    }
}

impl Unrooted<StructRef> {
    /// The type of the referenced struct.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn ty(&self, store: impl AsContext) -> Result<StructType> {
        let store = store.as_context().0;
        let index = self.type_index(store, VMGcKind::StructRef)?;
        match StructType::from_shared_type_index(store.engine(), index) {
            Some(ty) => Ok(ty),
            None => bail_bug!("invalid struct type index"),
        }
    }

    /// The value of the struct's `index`th field. `i8` and `i16` fields are
    /// zero-extended into `Val::I32`s.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid, or if the
    /// struct has no `index`th field.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn field(&self, store: impl AsContext, index: usize) -> Result<Val> {
        let store = store.as_context().0;
        let type_index = self.type_index(store, VMGcKind::StructRef)?;
        read_field(store, self.gc_ref(store)?, type_index, index, "struct")
    }

    /// The values of the struct's fields; see [`Unrooted::<StructRef>::field`].
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn fields(&self, store: impl AsContext) -> Result<Vec<Val>> {
        let store = store.as_context().0;
        let type_index = self.type_index(store, VMGcKind::StructRef)?;
        read_fields(store, self.gc_ref(store)?, type_index, "struct")
    }
}

impl Unrooted<ArrayRef> {
    /// The type of the referenced array.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn ty(&self, store: impl AsContext) -> Result<ArrayType> {
        let store = store.as_context().0;
        let index = self.type_index(store, VMGcKind::ArrayRef)?;
        match ArrayType::from_shared_type_index(store.engine(), index) {
            Some(ty) => Ok(ty),
            None => bail_bug!("invalid array type index"),
        }
    }

    /// The number of elements in the array.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn len(&self, store: impl AsContext) -> Result<u32> {
        let store = store.as_context().0;
        self.gc_ref(store)?.as_arrayref_unchecked().len(store)
    }

    /// The value of the array's `index`th element. `i8` and `i16` elements
    /// are zero-extended into `Val::I32`s.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid, or if `index` is
    /// out of bounds.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn get(&self, store: impl AsContext, index: u32) -> Result<Val> {
        let store = store.as_context().0;
        let gc_ref = self.gc_ref(store)?;
        let len = gc_ref.as_arrayref_unchecked().len(store)?;
        ensure!(
            index < len,
            "index out of bounds: the length is {len} but the index is {index}"
        );
        let type_index = self.type_index(store, VMGcKind::ArrayRef)?;
        let ty = match ArrayType::from_shared_type_index(store.engine(), type_index) {
            Some(ty) => ty,
            None => bail_bug!("invalid array type index"),
        };
        let layout = super::gc_array_layout(store.engine(), type_index)?;
        let Some(offset) = layout.elem_offset(index) else {
            bail_bug!("out-of-bounds array element index");
        };
        gc_ref.read_val(store, ty.field_type().element_type(), offset)
    }
}

impl Unrooted<ExnRef> {
    /// The type of the referenced exception.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn ty(&self, store: impl AsContext) -> Result<ExnType> {
        self._ty(store.as_context().0)
    }

    pub(crate) fn _ty(&self, store: &StoreOpaque) -> Result<ExnType> {
        let index = self.type_index(store, VMGcKind::ExnRef)?;
        match ExnType::from_shared_type_index(store.engine(), index) {
            Some(ty) => Ok(ty),
            None => bail_bug!("invalid exception type index"),
        }
    }

    /// The tag of the referenced exception.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn tag(&self, store: impl AsContext) -> Result<Tag> {
        let store = store.as_context().0;
        let (instance, index) = self.gc_ref(store)?.as_exnref_unchecked().tag(store)?;
        match Tag::from_raw_indices(store, instance, index) {
            Some(tag) => Ok(tag),
            None => bail_bug!("invalid tag indices in exception object"),
        }
    }

    /// The value of the exception's `index`th payload field.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid, or if the
    /// exception has no `index`th field.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn field(&self, store: impl AsContext, index: usize) -> Result<Val> {
        let store = store.as_context().0;
        let type_index = self.type_index(store, VMGcKind::ExnRef)?;
        read_field(store, self.gc_ref(store)?, type_index, index, "exn")
    }

    /// The values of the exception's payload fields.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn fields(&self, store: impl AsContext) -> Result<Vec<Val>> {
        let store = store.as_context().0;
        let type_index = self.type_index(store, VMGcKind::ExnRef)?;
        read_fields(store, self.gc_ref(store)?, type_index, "exn")
    }
}

impl Unrooted<ExternRef> {
    /// The host data of the referenced `externref`; see [`ExternRef::data`].
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn data<'a, T: 'static>(
        &self,
        store: impl Into<StoreContext<'a, T>>,
    ) -> Result<Option<&'a (dyn Any + Send + Sync)>> {
        let store = store.into().0;
        let gc_ref = self.gc_ref(store)?;
        if gc_ref.is_i31() {
            return Ok(None);
        }
        let gc_store = store.require_gc_store()?;
        match gc_ref.as_externref(&**gc_store.gc_heap.get()) {
            Some(externref) => Ok(Some(gc_store.externref_host_data(externref)?)),
            None => Ok(None),
        }
    }
}

/// The field types of the struct or exception type `type_index`.
fn field_types(
    store: &StoreOpaque,
    type_index: VMSharedTypeIndex,
) -> Result<Vec<crate::FieldType>> {
    if let Some(ty) = StructType::from_shared_type_index(store.engine(), type_index) {
        return Ok(ty.fields().collect());
    }
    match ExnType::from_shared_type_index(store.engine(), type_index) {
        Some(ty) => Ok(ty.fields().collect()),
        None => bail_bug!("invalid struct or exception type index"),
    }
}

/// Reads the `index`th field of the struct or exception `gc_ref`, of type
/// `type_index`.
fn read_field(
    store: &StoreOpaque,
    gc_ref: &VMGcRef,
    type_index: VMSharedTypeIndex,
    index: usize,
    what: &str,
) -> Result<Val> {
    let fields = field_types(store, type_index)?;
    let Some(field) = fields.get(index) else {
        let len = fields.len();
        bail!("cannot access field {index}: {what} only has {len} fields");
    };
    let layout = super::gc_struct_layout(store.engine(), type_index)?;
    gc_ref.read_val(store, field.element_type(), layout.fields[index].offset)
}

/// Reads every field of the struct or exception `gc_ref`, of type
/// `type_index`.
fn read_fields(
    store: &StoreOpaque,
    gc_ref: &VMGcRef,
    type_index: VMSharedTypeIndex,
    what: &str,
) -> Result<Vec<Val>> {
    let len = field_types(store, type_index)?.len();
    (0..len)
        .map(|i| read_field(store, gc_ref, type_index, i, what))
        .collect()
}
