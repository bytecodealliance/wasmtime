//! Working with GC `struct` objects.

use crate::runtime::vm::VMGcRef;
use crate::store::StoreId;
use crate::vm::{GcStructLayout, VMStructRef};
use crate::{
    prelude::*,
    store::{AutoAssertNoGc, StoreContextMut, StoreOpaque},
    AsContext, AsContextMut, GcHeapOutOfMemory, GcRefImpl, GcRootIndex, HeapType, ManuallyRooted,
    RefType, RootSet, Rooted, StructType, Val, ValRaw, ValType, WasmTy,
};
use core::mem::{self, MaybeUninit};
use wasmtime_environ::{VMGcKind, VMSharedTypeIndex};

/// An allocator for a particular Wasm GC struct type.
///
/// Every `StructRefPre` is associated with a particular
/// [`Store`][crate::Store] and a particular [StructType][crate::StructType].
///
/// Reusing an allocator across many allocations amortizes some per-type runtime
/// overheads inside Wasmtime. A `StructRefPre` is to `StructRef`s as an
/// `InstancePre` is to `Instance`s.
///
/// # Example
///
/// ```
/// use wasmtime::*;
///
/// # fn foo() -> Result<()> {
/// let mut config = Config::new();
/// config.wasm_function_references(true);
/// config.wasm_gc(true);
///
/// let engine = Engine::new(&config)?;
/// let mut store = Store::new(&engine, ());
///
/// // Define a struct type.
/// let struct_ty = StructType::new(
///    store.engine(),
///    [FieldType::new(Mutability::Var, StorageType::I8)],
/// )?;
///
/// // Create an allocator for the struct type.
/// let allocator = StructRefPre::new(&mut store, struct_ty);
///
/// {
///     let mut scope = RootScope::new(&mut store);
///
///     // Allocate a bunch of instances of our struct type using the same
///     // allocator! This is faster than creating a new allocator for each
///     // instance we want to allocate.
///     for i in 0..10 {
///         StructRef::new(&mut scope, &allocator, &[Val::I32(i)])?;
///     }
/// }
/// # Ok(())
/// # }
/// # foo().unwrap();
/// ```
pub struct StructRefPre {
    store_id: StoreId,
    ty: StructType,
}

impl StructRefPre {
    /// Create a new `StructRefPre` that is associated with the given store
    /// and type.
    pub fn new(mut store: impl AsContextMut, ty: StructType) -> Self {
        Self::_new(store.as_context_mut().0, ty)
    }

    pub(crate) fn _new(store: &mut StoreOpaque, ty: StructType) -> Self {
        store.insert_gc_host_alloc_type(ty.registered_type().clone());
        let store_id = store.id();

        StructRefPre { store_id, ty }
    }

    pub(crate) fn layout(&self) -> &GcStructLayout {
        self.ty
            .registered_type()
            .layout()
            .expect("struct types have a layout")
            .unwrap_struct()
    }

    pub(crate) fn type_index(&self) -> VMSharedTypeIndex {
        self.ty.registered_type().index()
    }
}

/// A reference to a GC-managed `struct` instance.
///
/// WebAssembly `struct`s are static, fixed-length, ordered sequences of
/// fields. Fields are named by index, not by identifier; in this way, they are
/// similar to Rust's tuples. Each field is mutable or constant and stores
/// unpacked [`Val`][crate::Val]s or packed 8-/16-bit integers.
///
/// Like all WebAssembly references, these are opaque and unforgeable to Wasm:
/// they cannot be faked and Wasm cannot, for example, cast the integer
/// `0x12345678` into a reference, pretend it is a valid `structref`, and trick
/// the host into dereferencing it and segfaulting or worse.
///
/// Note that you can also use `Rooted<StructRef>` and
/// `ManuallyRooted<StructRef>` as a type parameter with
/// [`Func::typed`][crate::Func::typed]- and
/// [`Func::wrap`][crate::Func::wrap]-style APIs.
///
/// # Example
///
/// ```
/// use wasmtime::*;
///
/// # fn foo() -> Result<()> {
/// let mut config = Config::new();
/// config.wasm_function_references(true);
/// config.wasm_gc(true);
///
/// let engine = Engine::new(&config)?;
/// let mut store = Store::new(&engine, ());
///
/// // Define a struct type.
/// let struct_ty = StructType::new(
///    store.engine(),
///    [FieldType::new(Mutability::Var, StorageType::I8)],
/// )?;
///
/// // Create an allocator for the struct type.
/// let allocator = StructRefPre::new(&mut store, struct_ty);
///
/// {
///     let mut scope = RootScope::new(&mut store);
///
///     // Allocate an instance of the struct type.
///     let my_struct = match StructRef::new(&mut scope, &allocator, &[Val::I32(42)]) {
///         Ok(s) => s,
///         // If the heap is out of memory, then do a GC and try again.
///         Err(e) if e.is::<GcHeapOutOfMemory<()>>() => {
///             // Do a GC! Note: in an async context, you'd want to do
///             // `scope.as_context_mut().gc_async().await`.
///             scope.as_context_mut().gc();
///
///             StructRef::new(&mut scope, &allocator, &[Val::I32(42)])?
///         }
///         Err(e) => return Err(e),
///     };
///
///     // That instance's field should have the expected value.
///     let val = my_struct.field(&mut scope, 0)?.unwrap_i32();
///     assert_eq!(val, 42);
///
///     // And we can update the field's value because it is a mutable field.
///     my_struct.set_field(&mut scope, 0, Val::I32(36))?;
///     let new_val = my_struct.field(&mut scope, 0)?.unwrap_i32();
///     assert_eq!(new_val, 36);
/// }
/// # Ok(())
/// # }
/// # foo().unwrap();
/// ```
#[derive(Debug)]
#[repr(transparent)]
pub struct StructRef {
    pub(super) inner: GcRootIndex,
}

unsafe impl GcRefImpl for StructRef {
    #[allow(private_interfaces)]
    fn transmute_ref(index: &GcRootIndex) -> &Self {
        // Safety: `StructRef` is a newtype of a `GcRootIndex`.
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

impl StructRef {
    /// Allocate a new `struct` and get a reference to it.
    ///
    /// # Errors
    ///
    /// If the given `fields` values' types do not match the field types of the
    /// `allocator`'s struct type, an error is returned.
    ///
    /// If the allocation cannot be satisfied because the GC heap is currently
    /// out of memory, but performing a garbage collection might free up space
    /// such that retrying the allocation afterwards might succeed, then a
    /// [`GcHeapOutOfMemory<()>`][crate::GcHeapOutOfMemory] error is returned.
    ///
    /// # Panics
    ///
    /// Panics if the allocator, or any of the field values, is not associated
    /// with the given store.
    pub fn new(
        mut store: impl AsContextMut,
        allocator: &StructRefPre,
        fields: &[Val],
    ) -> Result<Rooted<StructRef>> {
        Self::_new(store.as_context_mut().0, allocator, fields)
    }

    pub(crate) fn _new(
        store: &mut StoreOpaque,
        allocator: &StructRefPre,
        fields: &[Val],
    ) -> Result<Rooted<StructRef>> {
        assert_eq!(
            store.id(),
            allocator.store_id,
            "attempted to use a `StructRefPre` with the wrong store"
        );

        // Type check the given values against the field types.
        let expected_len = allocator.ty.fields().len();
        let actual_len = fields.len();
        ensure!(
            actual_len == expected_len,
            "expected {expected_len} fields, got {actual_len}"
        );
        for (ty, val) in allocator.ty.fields().zip(fields) {
            assert!(
                val.comes_from_same_store(store),
                "field value comes from the wrong store",
            );
            let ty = ty.element_type().unpack();
            val.ensure_matches_ty(store, ty)
                .context("field type mismatch")?;
        }

        // Allocate the struct and write each field value into the appropriate
        // offset.
        let structref = store
            .gc_store_mut()?
            .alloc_uninit_struct(allocator.type_index(), &allocator.layout())
            .err2anyhow()
            .context("unrecoverable error when allocating new `structref`")?
            .ok_or_else(|| GcHeapOutOfMemory::new(()))
            .err2anyhow()?;

        // From this point on, if we get any errors, then the struct is not
        // fully initialized, so we need to eagerly deallocate it before the
        // next GC where the collector might try to interpret one of the
        // uninitialized fields as a GC reference.
        let mut store = AutoAssertNoGc::new(store);
        match (|| {
            for (index, (ty, val)) in allocator.ty.fields().zip(fields).enumerate() {
                structref.initialize_field(
                    &mut store,
                    allocator.layout(),
                    ty.element_type(),
                    index,
                    val.clone(),
                )?;
            }
            Ok(())
        })() {
            Ok(()) => Ok(Rooted::new(&mut store, structref.into())),
            Err(e) => {
                store.gc_store_mut()?.dealloc_uninit_struct(structref);
                Err(e)
            }
        }
    }

    #[inline]
    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        self.inner.comes_from_same_store(store)
    }

    /// Get this `structref`'s type.
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn ty(&self, store: impl AsContext) -> Result<StructType> {
        self._ty(store.as_context().0)
    }

    pub(crate) fn _ty(&self, store: &StoreOpaque) -> Result<StructType> {
        assert!(self.comes_from_same_store(store));
        let index = self.type_index(store)?;
        Ok(StructType::from_shared_type_index(store.engine(), index))
    }

    /// Does this `structref` match the given type?
    ///
    /// That is, is this struct's type a subtype of the given type?
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store or if the
    /// type is not associated with the store's engine.
    pub fn matches_ty(&self, store: impl AsContext, ty: &StructType) -> Result<bool> {
        self._matches_ty(store.as_context().0, ty)
    }

    pub(crate) fn _matches_ty(&self, store: &StoreOpaque, ty: &StructType) -> Result<bool> {
        assert!(self.comes_from_same_store(store));
        Ok(self._ty(store)?.matches(ty))
    }

    pub(crate) fn ensure_matches_ty(&self, store: &StoreOpaque, ty: &StructType) -> Result<()> {
        if !self.comes_from_same_store(store) {
            bail!("function used with wrong store");
        }
        if self._matches_ty(store, ty)? {
            Ok(())
        } else {
            let actual_ty = self._ty(store)?;
            bail!("type mismatch: expected `(ref {ty})`, found `(ref {actual_ty})`")
        }
    }

    /// Get the values of this struct's fields.
    ///
    /// Note that `i8` and `i16` field values are zero-extended into
    /// `Val::I32(_)`s.
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn fields<'a, T: 'a>(
        &self,
        store: impl Into<StoreContextMut<'a, T>>,
    ) -> Result<impl ExactSizeIterator<Item = Val> + 'a> {
        self._fields(store.into().0)
    }

    pub(crate) fn _fields<'a>(
        &self,
        store: &'a mut StoreOpaque,
    ) -> Result<impl ExactSizeIterator<Item = Val> + 'a> {
        assert!(self.comes_from_same_store(store));

        let store = AutoAssertNoGc::new(store);
        let gc_ref = self.inner.try_gc_ref(&store)?.unchecked_copy();

        let header = store.gc_store()?.header(&gc_ref);
        debug_assert!(header.kind().matches(VMGcKind::StructRef));

        let index = header.ty().expect("structrefs should have concrete types");
        let ty = StructType::from_shared_type_index(store.engine(), index);

        let layout = store
            .engine()
            .signatures()
            .layout(index)
            .expect("struct types should have GC layouts");
        let layout = match layout {
            crate::vm::GcLayout::Array(_) => unreachable!(),
            crate::vm::GcLayout::Struct(s) => s,
        };

        let structref = gc_ref.into_structref_unchecked();
        debug_assert_eq!(ty.fields().len(), layout.fields.len());
        return Ok(Fields {
            store,
            structref,
            ty,
            layout,
            index: 0,
        });

        struct Fields<'a> {
            store: AutoAssertNoGc<'a>,
            structref: VMStructRef,
            ty: StructType,
            layout: GcStructLayout,
            index: usize,
        }

        impl Iterator for Fields<'_> {
            type Item = Val;

            fn next(&mut self) -> Option<Self::Item> {
                let ty = self.ty.field(self.index)?;
                let ty = ty.element_type();
                let i = self.index;
                self.index += 1;
                Some(
                    self.structref
                        .read_field(&mut self.store, &self.layout, ty, i),
                )
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let len = self.layout.fields.len() - self.index;
                (len, Some(len))
            }
        }

        impl ExactSizeIterator for Fields<'_> {}
    }

    /// Get this struct's `index`th field.
    ///
    /// Note that `i8` and `i16` field values are zero-extended into
    /// `Val::I32(_)`s.
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
        self._field(store.as_context_mut().0, index)
    }

    pub(crate) fn _field(&self, store: &mut StoreOpaque, index: usize) -> Result<Val> {
        assert!(self.comes_from_same_store(store));

        let mut fields = self._fields(store)?;
        let len = fields.len();
        fields
            .nth(index)
            .ok_or_else(|| anyhow!("cannot get field {index}: struct only has {len} fields"))
    }

    /// Set this struct's `index`th field.
    ///
    /// # Errors
    ///
    /// Returns an error in the following scenarios:
    ///
    /// * When given a value of the wrong type, such as trying to set an `f32`
    ///   field to an `i64` value.
    ///
    /// * When the field is not mutable.
    ///
    /// * When this struct does not have an `index`th field, i.e. `index` is out
    ///   of bounds.
    ///
    /// * When `value` is a GC reference that has since been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn set_field(&self, mut store: impl AsContextMut, index: usize, value: Val) -> Result<()> {
        self._set_field(store.as_context_mut().0, index, value)
    }

    pub(crate) fn _set_field(
        &self,
        store: &mut StoreOpaque,
        index: usize,
        value: Val,
    ) -> Result<()> {
        assert!(self.comes_from_same_store(store));

        let mut store = AutoAssertNoGc::new(store);
        let gc_ref = self.inner.try_gc_ref(&store)?.unchecked_copy();

        let header = store.gc_store()?.header(&gc_ref);
        debug_assert!(header.kind().matches(VMGcKind::StructRef));

        let ty_index = header.ty().expect("structrefs should have concrete types");
        let ty = StructType::from_shared_type_index(store.engine(), ty_index);

        let layout = store
            .engine()
            .signatures()
            .layout(ty_index)
            .expect("struct types should have GC layouts");
        let layout = match layout {
            crate::vm::GcLayout::Array(_) => unreachable!(),
            crate::vm::GcLayout::Struct(s) => s,
        };

        let structref = gc_ref.into_structref_unchecked();
        debug_assert_eq!(ty.fields().len(), layout.fields.len());

        let field_ty = match ty.field(index) {
            Some(ty) => ty,
            None => bail!(
                "cannot set field {index}: struct only has {} fields",
                ty.fields().len()
            ),
        };
        ensure!(
            field_ty.mutability().is_var(),
            "cannot set field {index}: field is not mutable"
        );
        value
            .ensure_matches_ty(&store, &field_ty.element_type().unpack())
            .with_context(|| format!("cannot set field {index}: type mismatch"))?;

        structref.write_field(&mut store, &layout, field_ty.element_type(), index, value)
    }

    pub(crate) fn type_index(&self, store: &StoreOpaque) -> Result<VMSharedTypeIndex> {
        let gc_ref = self.inner.unchecked_try_gc_ref(store)?;
        let header = store.gc_store()?.header(gc_ref);
        debug_assert!(header.kind().matches(VMGcKind::StructRef));
        Ok(header.ty().expect("structrefs should have concrete types"))
    }

    /// Create a new `Rooted<StructRef>` from the given GC reference.
    ///
    /// `gc_ref` should point to a valid `structref` and should belong to the
    /// store's GC heap. Failure to uphold these invariants is memory safe but
    /// will lead to general incorrectness such as panics or wrong results.
    pub(crate) fn from_cloned_gc_ref(
        store: &mut AutoAssertNoGc<'_>,
        gc_ref: VMGcRef,
    ) -> Rooted<Self> {
        debug_assert!(!gc_ref.is_i31());
        Rooted::new(store, gc_ref)
    }
}

unsafe impl WasmTy for Rooted<StructRef> {
    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Struct))
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.comes_from_same_store(store)
    }

    #[inline]
    fn dynamic_concrete_type_check(
        &self,
        store: &StoreOpaque,
        _nullable: bool,
        ty: &HeapType,
    ) -> Result<()> {
        match ty {
            HeapType::Any | HeapType::Eq | HeapType::Struct => Ok(()),
            HeapType::ConcreteStruct(ty) => self.ensure_matches_ty(store, ty),

            HeapType::Extern
            | HeapType::NoExtern
            | HeapType::Func
            | HeapType::ConcreteFunc(_)
            | HeapType::NoFunc
            | HeapType::I31
            | HeapType::Array
            | HeapType::ConcreteArray(_)
            | HeapType::None => bail!(
                "type mismatch: expected `(ref {ty})`, got `(ref {})`",
                self._ty(store)?,
            ),
        }
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        let gc_ref = self.inner.try_clone_gc_ref(store)?;
        let r64 = gc_ref.as_r64();
        store.gc_store_mut()?.expose_gc_ref_to_wasm(gc_ref);
        debug_assert_ne!(r64, 0);
        let anyref = u32::try_from(r64).unwrap();
        ptr.write(ValRaw::anyref(anyref));
        Ok(())
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        let raw = ptr.get_anyref();
        debug_assert_ne!(raw, 0);
        let gc_ref = VMGcRef::from_r64(raw.into())
            .expect("valid r64")
            .expect("non-null");
        let gc_ref = store.unwrap_gc_store_mut().clone_gc_ref(&gc_ref);
        StructRef::from_cloned_gc_ref(store, gc_ref)
    }
}

unsafe impl WasmTy for Option<Rooted<StructRef>> {
    #[inline]
    fn valtype() -> ValType {
        ValType::STRUCTREF
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.map_or(true, |x| x.comes_from_same_store(store))
    }

    #[inline]
    fn dynamic_concrete_type_check(
        &self,
        store: &StoreOpaque,
        nullable: bool,
        ty: &HeapType,
    ) -> Result<()> {
        match self {
            Some(s) => Rooted::<StructRef>::dynamic_concrete_type_check(s, store, nullable, ty),
            None => {
                ensure!(
                    nullable,
                    "expected a non-null reference, but found a null reference"
                );
                Ok(())
            }
        }
    }

    #[inline]
    fn is_vmgcref_and_points_to_object(&self) -> bool {
        self.is_some()
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        match self {
            Some(r) => r.store(store, ptr),
            None => {
                ptr.write(ValRaw::anyref(0));
                Ok(())
            }
        }
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        let gc_ref = VMGcRef::from_r64(ptr.get_anyref().into()).expect("valid r64")?;
        let gc_ref = store.unwrap_gc_store_mut().clone_gc_ref(&gc_ref);
        Some(StructRef::from_cloned_gc_ref(store, gc_ref))
    }
}

unsafe impl WasmTy for ManuallyRooted<StructRef> {
    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Struct))
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.comes_from_same_store(store)
    }

    #[inline]
    fn dynamic_concrete_type_check(
        &self,
        store: &StoreOpaque,
        _: bool,
        ty: &HeapType,
    ) -> Result<()> {
        match ty {
            HeapType::Any | HeapType::Eq | HeapType::Struct => Ok(()),
            HeapType::ConcreteStruct(ty) => self.ensure_matches_ty(store, ty),

            HeapType::Extern
            | HeapType::NoExtern
            | HeapType::Func
            | HeapType::ConcreteFunc(_)
            | HeapType::NoFunc
            | HeapType::I31
            | HeapType::Array
            | HeapType::ConcreteArray(_)
            | HeapType::None => bail!(
                "type mismatch: expected `(ref {ty})`, got `(ref {})`",
                self._ty(store)?,
            ),
        }
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        let gc_ref = self.inner.try_clone_gc_ref(store)?;
        let r64 = gc_ref.as_r64();
        store.gc_store_mut()?.expose_gc_ref_to_wasm(gc_ref);
        debug_assert_ne!(r64, 0);
        let anyref = u32::try_from(r64).unwrap();
        ptr.write(ValRaw::anyref(anyref));
        Ok(())
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        let raw = ptr.get_anyref();
        debug_assert_ne!(raw, 0);
        let gc_ref = VMGcRef::from_r64(raw.into())
            .expect("valid r64")
            .expect("non-null");
        let gc_ref = store.unwrap_gc_store_mut().clone_gc_ref(&gc_ref);
        RootSet::with_lifo_scope(store, |store| {
            let rooted = StructRef::from_cloned_gc_ref(store, gc_ref);
            rooted
                ._to_manually_rooted(store)
                .expect("rooted is in scope")
        })
    }
}

unsafe impl WasmTy for Option<ManuallyRooted<StructRef>> {
    #[inline]
    fn valtype() -> ValType {
        ValType::STRUCTREF
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.as_ref()
            .map_or(true, |x| x.comes_from_same_store(store))
    }

    #[inline]
    fn dynamic_concrete_type_check(
        &self,
        store: &StoreOpaque,
        nullable: bool,
        ty: &HeapType,
    ) -> Result<()> {
        match self {
            Some(s) => {
                ManuallyRooted::<StructRef>::dynamic_concrete_type_check(s, store, nullable, ty)
            }
            None => {
                ensure!(
                    nullable,
                    "expected a non-null reference, but found a null reference"
                );
                Ok(())
            }
        }
    }

    #[inline]
    fn is_vmgcref_and_points_to_object(&self) -> bool {
        self.is_some()
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        match self {
            Some(r) => r.store(store, ptr),
            None => {
                ptr.write(ValRaw::anyref(0));
                Ok(())
            }
        }
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        let raw = ptr.get_anyref();
        debug_assert_ne!(raw, 0);
        let gc_ref = VMGcRef::from_r64(raw.into()).expect("valid r64")?;
        let gc_ref = store.unwrap_gc_store_mut().clone_gc_ref(&gc_ref);
        RootSet::with_lifo_scope(store, |store| {
            let rooted = StructRef::from_cloned_gc_ref(store, gc_ref);
            Some(
                rooted
                    ._to_manually_rooted(store)
                    .expect("rooted is in scope"),
            )
        })
    }
}
