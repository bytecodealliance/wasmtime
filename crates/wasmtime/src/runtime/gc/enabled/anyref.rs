//! Implementation of `anyref` in Wasmtime.

use crate::prelude::*;
use crate::runtime::vm::VMGcRef;
use crate::{
    store::{AutoAssertNoGc, StoreOpaque},
    ArrayType, AsContext, AsContextMut, GcRefImpl, GcRootIndex, HeapType, ManuallyRooted, RefType,
    Result, Rooted, StructRef, StructType, ValRaw, ValType, WasmTy, I31,
};
use core::mem;
use core::mem::MaybeUninit;
use wasmtime_environ::VMGcKind;

/// An `anyref` GC reference.
///
/// The `AnyRef` type represents WebAssembly `anyref` values. These can be
/// references to `struct`s and `array`s or inline/unboxed 31-bit
/// integers. Unlike `externref`, Wasm guests can directly allocate `anyref`s.
///
/// Like all WebAssembly references, these are opaque and unforgable to Wasm:
/// they cannot be faked and Wasm cannot, for example, cast the integer
/// `0x12345678` into a reference, pretend it is a valid `anyref`, and trick the
/// host into dereferencing it and segfaulting or worse.
///
/// Note that you can also use `Rooted<AnyRef>` and `ManuallyRooted<AnyRef>` as
/// a type parameter with [`Func::typed`][crate::Func::typed]- and
/// [`Func::wrap`][crate::Func::wrap]-style APIs.
///
/// # Example
///
/// ```
/// # use wasmtime::*;
/// # fn _foo() -> Result<()> {
/// let mut config = Config::new();
/// config.wasm_gc(true);
///
/// let engine = Engine::new(&config)?;
///
/// // Define a module which does stuff with `anyref`s.
/// let module = Module::new(&engine, r#"
///     (module
///         (func (export "increment-if-i31") (param (ref null any)) (result (ref null any))
///             block
///                 ;; Try to cast the arg to an `i31`, otherwise branch out
///                 ;; of this `block`.
///                 local.get 0
///                 br_on_cast_fail (ref null any) (ref i31) 0
///                 ;; Get the `i31`'s inner value and add one to it.
///                 i31.get_u
///                 i32.const 1
///                 i32.add
///                 ;; Wrap the incremented value back into an `i31` reference and
///                 ;; return it.
///                 ref.i31
///                 return
///             end
///
///             ;; If the `anyref` we were given is not an `i31`, just return it
///             ;; as-is.
///             local.get 0
///         )
///     )
/// "#)?;
///
/// // Instantiate the module.
/// let mut store = Store::new(&engine, ());
/// let instance = Instance::new(&mut store, &module, &[])?;
///
/// // Extract the function.
/// let increment_if_i31 = instance
///     .get_typed_func::<Option<Rooted<AnyRef>>, Option<Rooted<AnyRef>>>(
///         &mut store,
///         "increment-if-i31",
///     )?;
///
/// {
///     // Create a new scope for the `Rooted` arguments and returns.
///     let mut scope = RootScope::new(&mut store);
///
///     // Call the function with an `i31`.
///     let arg = AnyRef::from_i31(&mut scope, I31::wrapping_u32(419));
///     let result = increment_if_i31.call(&mut scope, Some(arg))?;
///     assert_eq!(result.unwrap().as_i31(&scope)?, Some(I31::wrapping_u32(420)));
///
///     // Call the function with something that isn't an `i31`.
///     let result = increment_if_i31.call(&mut scope, None)?;
///     assert!(result.is_none());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
#[repr(transparent)]
pub struct AnyRef {
    pub(super) inner: GcRootIndex,
}

impl From<Rooted<StructRef>> for Rooted<AnyRef> {
    #[inline]
    fn from(s: Rooted<StructRef>) -> Self {
        s.to_anyref()
    }
}

impl From<ManuallyRooted<StructRef>> for ManuallyRooted<AnyRef> {
    #[inline]
    fn from(s: ManuallyRooted<StructRef>) -> Self {
        s.to_anyref()
    }
}

unsafe impl GcRefImpl for AnyRef {
    #[allow(private_interfaces)]
    fn transmute_ref(index: &GcRootIndex) -> &Self {
        // Safety: `AnyRef` is a newtype of a `GcRootIndex`.
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

impl AnyRef {
    /// Construct an `anyref` from an `i31`.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn _foo() -> Result<()> {
    /// let mut store = Store::<()>::default();
    ///
    /// // Create an `i31`.
    /// let i31 = I31::wrapping_u32(999);
    ///
    /// // Convert it into an `anyref`.
    /// let anyref = AnyRef::from_i31(&mut store, i31);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_i31(mut store: impl AsContextMut, value: I31) -> Rooted<Self> {
        let mut store = AutoAssertNoGc::new(store.as_context_mut().0);
        Self::_from_i31(&mut store, value)
    }

    pub(crate) fn _from_i31(store: &mut AutoAssertNoGc<'_>, value: I31) -> Rooted<Self> {
        let gc_ref = VMGcRef::from_i31(value.runtime_i31());
        Rooted::new(store, gc_ref)
    }

    /// Creates a new strongly-owned [`AnyRef`] from the raw value provided.
    ///
    /// This is intended to be used in conjunction with [`Func::new_unchecked`],
    /// [`Func::call_unchecked`], and [`ValRaw`] with its `anyref` field.
    ///
    /// This function assumes that `raw` is an `anyref` value which is currently
    /// rooted within the [`Store`].
    ///
    /// # Unsafety
    ///
    /// This function is particularly `unsafe` because `raw` not only must be a
    /// valid `anyref` value produced prior by [`AnyRef::to_raw`] but it must
    /// also be correctly rooted within the store. When arguments are provided
    /// to a callback with [`Func::new_unchecked`], for example, or returned via
    /// [`Func::call_unchecked`], if a GC is performed within the store then
    /// floating `anyref` values are not rooted and will be GC'd, meaning that
    /// this function will no longer be safe to call with the values cleaned up.
    /// This function must be invoked *before* possible GC operations can happen
    /// (such as calling Wasm).
    ///
    /// When in doubt try to not use this. Instead use the safe Rust APIs of
    /// [`TypedFunc`] and friends.
    ///
    /// [`Func::call_unchecked`]: crate::Func::call_unchecked
    /// [`Func::new_unchecked`]: crate::Func::new_unchecked
    /// [`Store`]: crate::Store
    /// [`TypedFunc`]: crate::TypedFunc
    /// [`ValRaw`]: crate::ValRaw
    pub unsafe fn from_raw(mut store: impl AsContextMut, raw: u32) -> Option<Rooted<Self>> {
        let mut store = AutoAssertNoGc::new(store.as_context_mut().0);
        Self::_from_raw(&mut store, raw)
    }

    // (Not actually memory unsafe since we have indexed GC heaps.)
    pub(crate) fn _from_raw(store: &mut AutoAssertNoGc, raw: u32) -> Option<Rooted<Self>> {
        let gc_ref = VMGcRef::from_raw_u32(raw)?;
        Some(Self::from_cloned_gc_ref(store, gc_ref))
    }

    /// Create a new `Rooted<AnyRef>` from the given GC reference.
    ///
    /// `gc_ref` should point to a valid `anyref` and should belong to the
    /// store's GC heap. Failure to uphold these invariants is memory safe but
    /// will lead to general incorrectness such as panics or wrong results.
    pub(crate) fn from_cloned_gc_ref(
        store: &mut AutoAssertNoGc<'_>,
        gc_ref: VMGcRef,
    ) -> Rooted<Self> {
        debug_assert!(
            gc_ref.is_i31()
                || store
                    .unwrap_gc_store()
                    .header(&gc_ref)
                    .kind()
                    .matches(VMGcKind::AnyRef)
        );
        Rooted::new(store, gc_ref)
    }

    #[inline]
    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        self.inner.comes_from_same_store(store)
    }

    /// Converts this [`AnyRef`] to a raw value suitable to store within a
    /// [`ValRaw`].
    ///
    /// Returns an error if this `anyref` has been unrooted.
    ///
    /// # Unsafety
    ///
    /// Produces a raw value which is only safe to pass into a store if a GC
    /// doesn't happen between when the value is produce and when it's passed
    /// into the store.
    ///
    /// [`ValRaw`]: crate::ValRaw
    pub unsafe fn to_raw(&self, mut store: impl AsContextMut) -> Result<u32> {
        let mut store = AutoAssertNoGc::new(store.as_context_mut().0);
        let gc_ref = self.inner.try_clone_gc_ref(&mut store)?;
        let raw = gc_ref.as_raw_u32();
        store.gc_store_mut()?.expose_gc_ref_to_wasm(gc_ref);
        Ok(raw)
    }

    /// Get the type of this reference.
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn ty(&self, store: impl AsContext) -> Result<HeapType> {
        self._ty(store.as_context().0)
    }

    pub(crate) fn _ty(&self, store: &StoreOpaque) -> Result<HeapType> {
        let gc_ref = self.inner.unchecked_try_gc_ref(store)?;
        if gc_ref.is_i31() {
            return Ok(HeapType::I31);
        }

        let header = store.gc_store()?.header(gc_ref);

        if header.kind().matches(VMGcKind::StructRef) {
            return Ok(HeapType::ConcreteStruct(
                StructType::from_shared_type_index(store.engine(), header.ty().unwrap()),
            ));
        }

        if header.kind().matches(VMGcKind::ArrayRef) {
            return Ok(HeapType::ConcreteArray(ArrayType::from_shared_type_index(
                store.engine(),
                header.ty().unwrap(),
            )));
        }

        unreachable!("no other kinds of `anyref`s")
    }

    /// Does this `anyref` match the given type?
    ///
    /// That is, is this object's type a subtype of the given type?
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn matches_ty(&self, store: impl AsContext, ty: &HeapType) -> Result<bool> {
        self._matches_ty(store.as_context().0, ty)
    }

    pub(crate) fn _matches_ty(&self, store: &StoreOpaque, ty: &HeapType) -> Result<bool> {
        assert!(self.comes_from_same_store(store));
        Ok(self._ty(store)?.matches(ty))
    }

    pub(crate) fn ensure_matches_ty(&self, store: &StoreOpaque, ty: &HeapType) -> Result<()> {
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

    /// Is this `anyref` an `i31`?
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn is_i31(&self, store: impl AsContext) -> Result<bool> {
        self._is_i31(store.as_context().0)
    }

    pub(crate) fn _is_i31(&self, store: &StoreOpaque) -> Result<bool> {
        assert!(self.comes_from_same_store(store));
        // NB: Can't use `AutoAssertNoGc` here because we only have a shared
        // context, not a mutable context.
        let gc_ref = self.inner.unchecked_try_gc_ref(store)?;
        Ok(gc_ref.is_i31())
    }

    /// Downcast this `anyref` to an `i31`.
    ///
    /// If this `anyref` is an `i31`, then `Some(_)` is returned.
    ///
    /// If this `anyref` is not an `i31`, then `None` is returned.
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn as_i31(&self, store: impl AsContext) -> Result<Option<I31>> {
        self._as_i31(store.as_context().0)
    }

    pub(crate) fn _as_i31(&self, store: &StoreOpaque) -> Result<Option<I31>> {
        assert!(self.comes_from_same_store(store));
        // NB: Can't use `AutoAssertNoGc` here because we only have a shared
        // context, not a mutable context.
        let gc_ref = self.inner.unchecked_try_gc_ref(store)?;
        Ok(gc_ref.as_i31().map(Into::into))
    }

    /// Downcast this `anyref` to an `i31`, panicking if this `anyref` is not an
    /// `i31`.
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store, or if
    /// this `anyref` is not an `i31`.
    pub fn unwrap_i31(&self, store: impl AsContext) -> Result<I31> {
        Ok(self.as_i31(store)?.expect("AnyRef::unwrap_i31 on non-i31"))
    }

    /// Is this `anyref` a `structref`?
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn is_struct(&self, store: impl AsContext) -> Result<bool> {
        self._is_struct(store.as_context().0)
    }

    pub(crate) fn _is_struct(&self, store: &StoreOpaque) -> Result<bool> {
        // NB: Can't use `AutoAssertNoGc` here because we only have a shared
        // context, not a mutable context.
        let gc_ref = self.inner.unchecked_try_gc_ref(store)?;
        Ok(!gc_ref.is_i31() && store.gc_store()?.kind(gc_ref).matches(VMGcKind::StructRef))
    }

    /// Downcast this `anyref` to a `structref`.
    ///
    /// If this `anyref` is a `structref`, then `Some(_)` is returned.
    ///
    /// If this `anyref` is not a `structref`, then `None` is returned.
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn as_struct(&self, store: impl AsContext) -> Result<Option<Rooted<StructRef>>> {
        self._as_struct(store.as_context().0)
    }

    pub(crate) fn _as_struct(&self, store: &StoreOpaque) -> Result<Option<Rooted<StructRef>>> {
        if self._is_struct(store)? {
            Ok(Some(Rooted::from_gc_root_index(self.inner)))
        } else {
            Ok(None)
        }
    }

    /// Downcast this `anyref` to an `i31`, panicking if this `anyref` is not a
    /// `struct`.
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store, or if
    /// this `anyref` is not a `struct`.
    pub fn unwrap_struct(&self, store: impl AsContext) -> Result<Rooted<StructRef>> {
        self._unwrap_struct(store.as_context().0)
    }

    pub(crate) fn _unwrap_struct(&self, store: &StoreOpaque) -> Result<Rooted<StructRef>> {
        Ok(self
            ._as_struct(store)?
            .expect("AnyRef::unwrap_struct on non-structref"))
    }
}

unsafe impl WasmTy for Rooted<AnyRef> {
    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Any))
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
        self.ensure_matches_ty(store, ty)
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        self.wasm_ty_store(store, ptr, ValRaw::anyref)
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        Self::wasm_ty_load(store, ptr.get_anyref(), AnyRef::from_cloned_gc_ref)
    }
}

unsafe impl WasmTy for Option<Rooted<AnyRef>> {
    #[inline]
    fn valtype() -> ValType {
        ValType::ANYREF
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
            Some(a) => a.ensure_matches_ty(store, ty),
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
        <Rooted<AnyRef>>::wasm_ty_option_store(self, store, ptr, ValRaw::anyref)
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        <Rooted<AnyRef>>::wasm_ty_option_load(store, ptr.get_anyref(), AnyRef::from_cloned_gc_ref)
    }
}

unsafe impl WasmTy for ManuallyRooted<AnyRef> {
    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Any))
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
        self.ensure_matches_ty(store, ty)
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        self.wasm_ty_store(store, ptr, ValRaw::anyref)
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        Self::wasm_ty_load(store, ptr.get_anyref(), AnyRef::from_cloned_gc_ref)
    }
}

unsafe impl WasmTy for Option<ManuallyRooted<AnyRef>> {
    #[inline]
    fn valtype() -> ValType {
        ValType::ANYREF
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
            Some(a) => a.ensure_matches_ty(store, ty),
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
        <ManuallyRooted<AnyRef>>::wasm_ty_option_store(self, store, ptr, ValRaw::anyref)
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        <ManuallyRooted<AnyRef>>::wasm_ty_option_load(
            store,
            ptr.get_anyref(),
            AnyRef::from_cloned_gc_ref,
        )
    }
}
