//! Frozen references: cheap, weak, read-only references to GC objects, which
//! the host API's accessors return.

use crate::runtime::vm::{GcEpoch, VMGcHeader, VMGcRef};
use crate::store::{AutoAssertNoGc, StoreId, StoreOpaque};
use crate::{
    AnyRef, ArrayRef, ArrayType, AsContext, AsContextMut, ExnRef, ExnType, ExternRef, GcRef,
    HeapType, I31, OwnedRooted, Result, Rooted, StoreContext, StructRef, StructType, Tag, Val,
    bail_bug, prelude::*,
};
use core::any::Any;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker;
use core::num::NonZeroU32;
use wasmtime_environ::{VMGcKind, VMSharedTypeIndex};

/// A weak, read-only reference to a GC object, valid until the store's GC
/// heap or execution next changes.
///
/// Accessors that read GC references out of the store return them frozen, as
/// [`Val::Frozen`] or [`Ref::Frozen`](crate::Ref::Frozen) values: for example
/// [`Global::get`](crate::Global::get), [`Table::get`](crate::Table::get),
/// [`StructRef::field`], [`ArrayRef::get`], [`ExnRef::field`], and
/// [`FrameHandle::local`](crate::FrameHandle::local). A frozen reference is
/// just the object's location and a snapshot of the store's GC epoch, so it is
/// cheap to create, and creating and reading one needs only shared access to
/// the store ([`AsContext`]).
///
/// Unlike [`Rooted`] and [`OwnedRooted`], a `Frozen` reference neither keeps
/// its object alive nor follows it when a collector moves it. Instead, the
/// store's GC heap has an epoch, which every entry into and exit from Wasm,
/// every host-API mutation of a GC object, and anything else that may free or
/// move objects (such as a collection) advances. A frozen reference is valid
/// only during the epoch in which it was created; using it afterwards returns
/// an error. Frozen references to `i31`s, which are not objects in the heap,
/// stay valid.
///
/// Rooting is a separate, explicit step: to keep an object alive and usable
/// past the current epoch, root it with [`Frozen::to_rooted`] (in the current
/// LIFO rooting scope) or [`Frozen::to_owned_rooted`]. APIs that take GC
/// references accept valid frozen references as well as rooted ones.
///
/// Frozen references compare equal, and hash the same, exactly when they are
/// valid at the same time and refer to the same object (or are equal
/// `i31`s).
pub struct Frozen<T: GcRef> {
    store_id: StoreId,
    /// The store's `Self::epoch` when this reference was created; unused, and
    /// always `GcEpoch::INITIAL`, for `i31`s.
    epoch: GcEpoch,
    raw: NonZeroU32,
    _phantom: marker::PhantomData<fn() -> T>,
}

impl<T: GcRef> Clone for Frozen<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: GcRef> Copy for Frozen<T> {}

impl<T: GcRef> fmt::Debug for Frozen<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format!("Frozen<{}>", core::any::type_name::<T>());
        f.debug_struct(&name)
            .field("store_id", &self.store_id)
            .field("epoch", &self.epoch)
            .field("raw", &self.raw)
            .finish()
    }
}

impl<T: GcRef> PartialEq for Frozen<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.store_id, self.epoch, self.raw) == (other.store_id, other.epoch, other.raw)
    }
}

impl<T: GcRef> Eq for Frozen<T> {}

impl<T: GcRef> Hash for Frozen<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.store_id, self.epoch, self.raw).hash(state);
    }
}

impl<T: GcRef> Frozen<T> {
    /// A frozen reference to `gc_ref`, which must be a `T` in `store`.
    pub(crate) fn new(store: &StoreOpaque, gc_ref: &VMGcRef) -> Self {
        let epoch = if gc_ref.is_i31() {
            GcEpoch::INITIAL
        } else {
            Self::epoch(store)
        };
        Frozen {
            store_id: store.id(),
            epoch,
            raw: gc_ref.as_raw_non_zero_u32(),
            _phantom: marker::PhantomData,
        }
    }

    /// The referenced object, if this reference is still valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub(crate) fn gc_ref(&self, store: &StoreOpaque) -> Result<VMGcRef> {
        assert!(
            self.store_id == store.id(),
            "frozen reference used with the wrong store"
        );
        let gc_ref = VMGcRef::from_raw_non_zero_u32(self.raw);
        if !gc_ref.is_i31() {
            ensure!(
                Self::epoch(store) == self.epoch,
                "attempted to use a frozen garbage-collected reference after \
                 the GC heap or execution changed"
            );
        }
        Ok(gc_ref)
    }

    /// The store's GC epoch; see `GcStore::frozen_epoch`.
    fn epoch(store: &StoreOpaque) -> GcEpoch {
        store.unwrap_gc_store().frozen_epoch
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

    fn cast<U: GcRef>(self) -> Frozen<U> {
        Frozen {
            store_id: self.store_id,
            epoch: self.epoch,
            raw: self.raw,
            _phantom: marker::PhantomData,
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        self.store_id == store.id()
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
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn to_rooted(&self, mut store: impl AsContextMut) -> Result<Rooted<T>> {
        let mut store = AutoAssertNoGc::new(store.as_context_mut().0);
        let gc_ref = self.gc_ref(&store)?;
        let gc_ref = store.clone_gc_ref(&gc_ref);
        Ok(Rooted::new(&mut store, gc_ref))
    }

    /// Roots the referenced object until the returned handle is dropped, to
    /// keep it alive and usable after this reference becomes invalid; see
    /// [`OwnedRooted`].
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
        let mut store = AutoAssertNoGc::new(store.as_context_mut().0);
        let gc_ref = self.gc_ref(&store)?;
        let gc_ref = store.clone_gc_ref(&gc_ref);
        Ok(OwnedRooted::new(&mut store, gc_ref)?)
    }
}

impl Frozen<AnyRef> {
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
        AnyRef::ty_of(store, &self.gc_ref(store)?)
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
    pub fn as_struct(&self, store: impl AsContext) -> Result<Option<Frozen<StructRef>>> {
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
    pub fn as_array(&self, store: impl AsContext) -> Result<Option<Frozen<ArrayRef>>> {
        let kind = self.kind(store.as_context().0)?;
        Ok(kind
            .is_some_and(|k| k.matches(VMGcKind::ArrayRef))
            .then(|| self.cast()))
    }
}

impl From<Frozen<StructRef>> for Frozen<AnyRef> {
    fn from(r: Frozen<StructRef>) -> Self {
        r.cast()
    }
}

impl From<Frozen<ArrayRef>> for Frozen<AnyRef> {
    fn from(r: Frozen<ArrayRef>) -> Self {
        r.cast()
    }
}

impl Frozen<StructRef> {
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

    /// The values of the struct's fields; see [`Frozen::<StructRef>::field`].
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

impl Frozen<ArrayRef> {
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

impl Frozen<ExnRef> {
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

impl Frozen<ExternRef> {
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
        match gc_ref.as_externref(&*gc_store.gc_heap) {
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
    gc_ref: VMGcRef,
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
    gc_ref: VMGcRef,
    type_index: VMSharedTypeIndex,
    what: &str,
) -> Result<Vec<Val>> {
    let len = field_types(store, type_index)?.len();
    (0..len)
        .map(|i| read_field(store, gc_ref.unchecked_copy(), type_index, i, what))
        .collect()
}
