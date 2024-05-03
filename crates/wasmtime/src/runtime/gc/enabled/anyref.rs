//! Implementation of `anyref` in Wasmtime.

use crate::runtime::vm::VMGcRef;
use crate::{
    store::{AutoAssertNoGc, StoreOpaque},
    AsContext, AsContextMut, GcRefImpl, GcRootIndex, HeapType, ManuallyRooted, RefType, Result,
    RootSet, Rooted, ValRaw, ValType, WasmTy, I31,
};
use core::mem;
use core::num::NonZeroU64;

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
    inner: GcRootIndex,
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
        let gc_ref = VMGcRef::from_raw_u32(raw)?;
        Some(Self::from_cloned_gc_ref(&mut store, gc_ref))
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
        assert!(gc_ref.is_i31());
        assert!(VMGcRef::ONLY_EXTERN_REF_AND_I31);
        Rooted::new(store, gc_ref)
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

    /// Is this `anyref` an `i31`?
    ///
    /// Returns an `Err(_)` if this reference has been unrooted.
    pub fn is_i31(&self, store: impl AsContext) -> Result<bool> {
        self._is_i31(store.as_context().0)
    }

    pub(crate) fn _is_i31(&self, store: &StoreOpaque) -> Result<bool> {
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
    /// Returns an `Err(_)` if this reference has been unrooted.
    pub fn as_i31(&self, store: impl AsContext) -> Result<Option<I31>> {
        // NB: Can't use `AutoAssertNoGc` here because we only have a shared
        // context, not a mutable context.
        let gc_ref = self.inner.unchecked_try_gc_ref(store.as_context().0)?;
        Ok(gc_ref.as_i31().map(Into::into))
    }

    /// Downcast this `anyref` to an `i31`, panicking if this `anyref` is not an
    /// `i31`.
    ///
    /// Returns an `Err(_)` if this reference has been unrooted.
    pub fn unwrap_i31(&self, store: impl AsContext) -> Result<I31> {
        Ok(self.as_i31(store)?.expect("AnyRef::unwrap_i31 on non-i31"))
    }
}

unsafe impl WasmTy for Rooted<AnyRef> {
    // TODO: this should be `VMGcRef` but Cranelift currently doesn't support
    // using r32 types when targeting 64-bit platforms.
    type Abi = NonZeroU64;

    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Any))
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.comes_from_same_store(store)
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
        unreachable!()
    }

    #[inline]
    fn is_non_i31_gc_ref(&self) -> bool {
        true
    }

    #[inline]
    unsafe fn abi_from_raw(raw: *mut ValRaw) -> Self::Abi {
        let raw = (*raw).get_externref();
        debug_assert_ne!(raw, 0);
        NonZeroU64::new_unchecked(u64::from(raw))
    }

    #[inline]
    unsafe fn abi_into_raw(abi: Self::Abi, raw: *mut ValRaw) {
        let externref = u32::try_from(abi.get()).unwrap();
        *raw = ValRaw::externref(externref);
    }

    #[inline]
    fn into_abi(self, store: &mut AutoAssertNoGc<'_>) -> Result<Self::Abi> {
        let gc_ref = self.inner.try_clone_gc_ref(store)?;
        let r64 = gc_ref.as_r64();
        store.gc_store_mut()?.expose_gc_ref_to_wasm(gc_ref);
        debug_assert_ne!(r64, 0);
        Ok(unsafe { NonZeroU64::new_unchecked(r64) })
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, store: &mut AutoAssertNoGc<'_>) -> Self {
        let gc_ref = VMGcRef::from_r64(abi.get())
            .expect("valid r64")
            .expect("non-null");
        let gc_ref = store.unwrap_gc_store_mut().clone_gc_ref(&gc_ref);
        AnyRef::from_cloned_gc_ref(store, gc_ref)
    }
}

unsafe impl WasmTy for Option<Rooted<AnyRef>> {
    type Abi = u64;

    #[inline]
    fn valtype() -> ValType {
        ValType::ANYREF
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.map_or(true, |x| x.comes_from_same_store(store))
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
        unreachable!()
    }

    #[inline]
    fn is_non_i31_gc_ref(&self) -> bool {
        true
    }

    #[inline]
    unsafe fn abi_from_raw(raw: *mut ValRaw) -> Self::Abi {
        let externref = (*raw).get_externref();
        u64::from(externref)
    }

    #[inline]
    unsafe fn abi_into_raw(abi: Self::Abi, raw: *mut ValRaw) {
        let externref = u32::try_from(abi).unwrap();
        *raw = ValRaw::externref(externref);
    }

    #[inline]
    fn into_abi(self, store: &mut AutoAssertNoGc<'_>) -> Result<Self::Abi> {
        Ok(if let Some(x) = self {
            <Rooted<AnyRef> as WasmTy>::into_abi(x, store)?.get()
        } else {
            0
        })
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, store: &mut AutoAssertNoGc<'_>) -> Self {
        let gc_ref = VMGcRef::from_r64(abi).expect("valid r64")?;
        let gc_ref = store.unwrap_gc_store_mut().clone_gc_ref(&gc_ref);
        Some(AnyRef::from_cloned_gc_ref(store, gc_ref))
    }
}

unsafe impl WasmTy for ManuallyRooted<AnyRef> {
    type Abi = NonZeroU64;

    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Any))
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.comes_from_same_store(store)
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
        unreachable!()
    }

    #[inline]
    fn is_non_i31_gc_ref(&self) -> bool {
        true
    }

    #[inline]
    unsafe fn abi_from_raw(raw: *mut ValRaw) -> Self::Abi {
        let externref = (*raw).get_externref();
        debug_assert_ne!(externref, 0);
        NonZeroU64::new_unchecked(u64::from(externref))
    }

    #[inline]
    unsafe fn abi_into_raw(abi: Self::Abi, raw: *mut ValRaw) {
        let externref = u32::try_from(abi.get()).unwrap();
        *raw = ValRaw::externref(externref);
    }

    #[inline]
    fn into_abi(self, store: &mut AutoAssertNoGc<'_>) -> Result<Self::Abi> {
        let gc_ref = self.inner.try_clone_gc_ref(store)?;
        let r64 = gc_ref.as_r64();
        store.gc_store_mut()?.expose_gc_ref_to_wasm(gc_ref);
        Ok(unsafe { NonZeroU64::new_unchecked(r64) })
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, store: &mut AutoAssertNoGc<'_>) -> Self {
        let gc_ref = VMGcRef::from_r64(abi.get())
            .expect("valid r64")
            .expect("non-null");
        let gc_ref = store.unwrap_gc_store_mut().clone_gc_ref(&gc_ref);
        RootSet::with_lifo_scope(store, |store| {
            let rooted = AnyRef::from_cloned_gc_ref(store, gc_ref);
            rooted
                ._to_manually_rooted(store)
                .expect("rooted is in scope")
        })
    }
}

unsafe impl WasmTy for Option<ManuallyRooted<AnyRef>> {
    type Abi = u64;

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
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
        unreachable!()
    }

    #[inline]
    fn is_non_i31_gc_ref(&self) -> bool {
        true
    }

    #[inline]
    unsafe fn abi_from_raw(raw: *mut ValRaw) -> Self::Abi {
        let externref = (*raw).get_externref();
        u64::from(externref)
    }

    #[inline]
    unsafe fn abi_into_raw(abi: Self::Abi, raw: *mut ValRaw) {
        let externref = u32::try_from(abi).unwrap();
        *raw = ValRaw::externref(externref);
    }

    #[inline]
    fn into_abi(self, store: &mut AutoAssertNoGc<'_>) -> Result<Self::Abi> {
        Ok(if let Some(x) = self {
            <ManuallyRooted<AnyRef> as WasmTy>::into_abi(x, store)?.get()
        } else {
            0
        })
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, store: &mut AutoAssertNoGc<'_>) -> Self {
        let gc_ref = VMGcRef::from_r64(abi).expect("valid r64")?;
        let gc_ref = store.unwrap_gc_store_mut().clone_gc_ref(&gc_ref);
        RootSet::with_lifo_scope(store, |store| {
            let rooted = AnyRef::from_cloned_gc_ref(store, gc_ref);
            Some(
                rooted
                    ._to_manually_rooted(store)
                    .expect("rooted is in scope"),
            )
        })
    }
}
