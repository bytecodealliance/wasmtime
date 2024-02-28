//! Implementation of `externref` in Wasmtime.

use crate::{
    store::{AutoAssertNoGc, StoreOpaque},
    AsContextMut, FuncType, GcHeapOutOfMemory, GcRefImpl, GcRootIndex, HeapType, ManuallyRooted,
    RefType, Result, RootSet, Rooted, StoreContext, StoreContextMut, ValRaw, ValType, WasmTy,
};
use anyhow::Context;
use std::any::Any;
use std::num::NonZeroU64;
use wasmtime_runtime::VMGcRef;

/// An opaque, GC-managed reference to some host data that can be passed to
/// WebAssembly.
///
/// The `ExternRef` type represents WebAssembly `externref` values. Wasm can't
/// do anything with the `externref`s other than put them in tables, globals,
/// and locals or pass them to other functions (such as imported functions from
/// the host). Unlike `anyref`s, Wasm guests cannot directly allocate new
/// `externref`s; only the host can.
///
/// You can use `ExternRef` to give access to host objects and control the
/// operations that Wasm can perform on them via what functions you allow Wasm
/// to import.
///
/// Like all WebAssembly references, these are opaque and unforgable to Wasm:
/// they cannot be faked and Wasm cannot, for example, cast the integer
/// `0x12345678` into a reference, pretend it is a valid `externref`, and trick
/// the host into dereferencing it and segfaulting or worse.
///
/// Note that you can also use `Rooted<ExternRef>` and
/// `ManuallyRooted<ExternRef>` as a type parameter with
/// [`Func::typed`][crate::Func::typed]- and
/// [`Func::wrap`][crate::Func::wrap]-style APIs.
///
/// # Example
///
/// ```
/// # use wasmtime::*;
/// # use std::borrow::Cow;
/// # fn _foo() -> Result<()> {
/// let engine = Engine::default();
/// let mut store = Store::new(&engine, ());
///
/// // Define some APIs for working with host strings from Wasm via `externref`.
/// let mut linker = Linker::new(&engine);
/// linker.func_wrap(
///     "host-string",
///     "new",
///     |caller: Caller<'_, ()>| -> Result<Rooted<ExternRef>> {
///         ExternRef::new(caller, Cow::from(""))
///     },
/// )?;
/// linker.func_wrap(
///     "host-string",
///     "concat",
///     |mut caller: Caller<'_, ()>, a: Rooted<ExternRef>, b: Rooted<ExternRef>| -> Result<Rooted<ExternRef>> {
///         let mut s = a
///             .data(&caller)?
///             .downcast_ref::<Cow<str>>()
///             .ok_or_else(|| Error::msg("externref was not a string"))?
///             .clone()
///             .into_owned();
///         let b = b
///             .data(&caller)?
///             .downcast_ref::<Cow<str>>()
///             .ok_or_else(|| Error::msg("externref was not a string"))?;
///         s.push_str(&b);
///         ExternRef::new(&mut caller, s)
///     },
/// )?;
///
/// // Here is a Wasm module that uses those APIs.
/// let module = Module::new(
///     &engine,
///     r#"
///         (module
///             (import "host-string" "concat" (func $concat (param externref externref)
///                                                          (result externref)))
///             (func (export "run") (param externref externref) (result externref)
///                 local.get 0
///                 local.get 1
///                 call $concat
///             )
///         )
///     "#,
/// )?;
///
/// // Create a couple `externref`s wrapping `Cow<str>`s.
/// let hello = ExternRef::new(&mut store, Cow::from("Hello, "))?;
/// let world = ExternRef::new(&mut store, Cow::from("World!"))?;
///
/// // Instantiate the module and pass the `externref`s into it.
/// let instance = linker.instantiate(&mut store, &module)?;
/// let result = instance
///     .get_typed_func::<(Rooted<ExternRef>, Rooted<ExternRef>), Rooted<ExternRef>>(&mut store, "run")?
///     .call(&mut store, (hello, world))?;
///
/// // The module should have concatenated the strings together!
/// assert_eq!(
///     result.data(&store)?.downcast_ref::<Cow<str>>().unwrap(),
///     "Hello, World!"
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct ExternRef {
    inner: GcRootIndex,
}

unsafe impl GcRefImpl for ExternRef {
    #[allow(private_interfaces)]
    fn transmute_ref(index: &GcRootIndex) -> &Self {
        // Safety: `ExternRef` is a newtype of a `GcRootIndex`.
        let me: &Self = unsafe { std::mem::transmute(index) };

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

impl ExternRef {
    /// Creates a new instance of `ExternRef` wrapping the given value.
    ///
    /// The resulting value is automatically unrooted when the given `context`'s
    /// scope is exited. See [`Rooted<T>`][crate::Rooted]'s documentation for
    /// more details.
    ///
    /// This method will *not* automatically trigger a GC to free up space in
    /// the GC heap; instead it will return an error. This gives you more
    /// precise control over when collections happen and allows you to choose
    /// between performing synchronous and asynchronous collections.
    ///
    /// # Errors
    ///
    /// If the allocation cannot be satisfied because the GC heap is currently
    /// out of memory, but performing a garbage collection might free up space
    /// such that retrying the allocation afterwards might succeed, then a
    /// `GcHeapOutOfMemory<T>` error is returned.
    ///
    /// The `GcHeapOutOfMemory<T>` error contains the host value that the
    /// `externref` would have wrapped. You can extract that value from this
    /// error and reuse it when attempting to allocate an `externref` again
    /// after GC or otherwise do with it whatever you see fit.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn _foo() -> Result<()> {
    /// let mut store = Store::<()>::default();
    ///
    /// {
    ///     let mut scope = RootScope::new(&mut store);
    ///
    ///     // Create an `externref` wrapping a `str`.
    ///     let externref = match ExternRef::new(&mut scope, "hello!") {
    ///         Ok(x) => x,
    ///         // If the heap is out of memory, then do a GC and try again.
    ///         Err(e) if e.is::<GcHeapOutOfMemory<&'static str>>() => {
    ///             // Do a GC! Note: in an async context, you'd want to do
    ///             // `scope.as_context_mut().gc_async().await`.
    ///             scope.as_context_mut().gc();
    ///
    ///             // Extract the original host value from the error.
    ///             let host_value = e
    ///                 .downcast::<GcHeapOutOfMemory<&'static str>>()
    ///                 .unwrap()
    ///                 .into_inner();
    ///
    ///             // Try to allocate the `externref` again, now that the GC
    ///             // has hopefully freed up some space.
    ///             ExternRef::new(&mut scope, host_value)?
    ///         }
    ///         Err(e) => return Err(e),
    ///     };
    ///
    ///     // Use the `externref`, pass it to Wasm, etc...
    /// }
    ///
    /// // The `externref` is automatically unrooted when we exit the scope.
    /// # Ok(())
    /// # }
    /// ```
    pub fn new<T>(mut context: impl AsContextMut, value: T) -> Result<Rooted<ExternRef>>
    where
        T: 'static + Any + Send + Sync,
    {
        let ctx = context.as_context_mut().0;

        let value: Box<dyn Any + Send + Sync> = Box::new(value);
        let gc_ref = ctx
            .gc_store_mut()?
            .alloc_externref(value)
            .context("unrecoverable error when allocating new `externref`")?
            .map_err(|x| GcHeapOutOfMemory::<T>::new(*x.downcast().unwrap()))
            .context("failed to allocate `externref`")?;

        let mut ctx = AutoAssertNoGc::new(ctx);
        Ok(Self::from_cloned_gc_ref(&mut ctx, gc_ref.into()))
    }

    /// Creates a new, manually-rooted instance of `ExternRef` wrapping the
    /// given value.
    ///
    /// The resulting value must be manually unrooted, or else it will leak for
    /// the entire duration of the store's lifetime. See
    /// [`ManuallyRooted<T>`][crate::ManuallyRooted]'s documentation for more
    /// details.
    ///
    /// # Errors
    ///
    /// This function returns the same errors in the same scenarios as
    /// [`ExternRef::new`][crate::ExternRef::new].
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn _foo() -> Result<()> {
    /// let mut store = Store::<()>::default();
    ///
    /// // Create a manually-rooted `externref` wrapping a `str`.
    /// let externref = ExternRef::new_manually_rooted(&mut store, "hello!")?;
    ///
    /// // Use `externref` a bunch, pass it to Wasm, etc...
    ///
    /// // Don't forget to explicitly unroot the `externref` when you're done
    /// // using it!
    /// externref.unroot(&mut store);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_manually_rooted<T>(
        mut store: impl AsContextMut,
        value: T,
    ) -> Result<ManuallyRooted<ExternRef>>
    where
        T: 'static + Any + Send + Sync,
    {
        let ctx = store.as_context_mut().0;

        let value: Box<dyn Any + Send + Sync> = Box::new(value);
        let gc_ref = ctx
            .gc_store_mut()?
            .alloc_externref(value)
            .context("unrecoverable error when allocating new `externref`")?
            .map_err(|x| GcHeapOutOfMemory::<T>::new(*x.downcast().unwrap()))
            .context("failed to allocate `externref`")?;

        let mut ctx = AutoAssertNoGc::new(ctx);
        Ok(ManuallyRooted::new(&mut ctx, gc_ref.into()))
    }

    /// Create a new `Rooted<ExternRef>` from the given GC reference.
    ///
    /// Does not invoke the `GcRuntime`'s clone hook; callers should ensure it
    /// has been called.
    ///
    /// `gc_ref` should be a GC reference pointing to an instance of `externref`
    /// that is in this store's GC heap. Failure to uphold this invariant is
    /// memory safe but will result in general incorrectness such as panics and
    /// wrong results.
    pub(crate) fn from_cloned_gc_ref(
        store: &mut AutoAssertNoGc<'_>,
        gc_ref: VMGcRef,
    ) -> Rooted<Self> {
        assert!(
            gc_ref.is_extern_ref(),
            "GC reference {gc_ref:#p} is not an externref"
        );
        Rooted::new(store, gc_ref)
    }

    /// Get a shared borrow of the underlying data for this `ExternRef`.
    ///
    /// Returns an error if this `externref` GC reference has been unrooted (eg
    /// if you attempt to use a `Rooted<ExternRef>` after exiting the scope it
    /// was rooted within). See the documentation for
    /// [`Rooted<T>`][crate::Rooted] for more details.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn _foo() -> Result<()> {
    /// let mut store = Store::<()>::default();
    ///
    /// let externref = ExternRef::new(&mut store, "hello")?;
    ///
    /// // Access the `externref`'s host data.
    /// let data = externref.data(&store)?;
    /// // Dowcast it to a `&str`.
    /// let data = data.downcast_ref::<&str>().ok_or_else(|| Error::msg("not a str"))?;
    /// // We should have got the data we created the `externref` with!
    /// assert_eq!(*data, "hello");
    /// # Ok(())
    /// # }
    /// ```
    pub fn data<'a, T>(
        &self,
        store: impl Into<StoreContext<'a, T>>,
    ) -> Result<&'a (dyn Any + Send + Sync)>
    where
        T: 'a,
    {
        let store = store.into().0;
        let gc_ref = self.inner.unchecked_try_gc_ref(&store)?;
        let externref = gc_ref.as_externref_unchecked();
        Ok(store.gc_store()?.externref_host_data(externref))
    }

    /// Get an exclusive borrow of the underlying data for this `ExternRef`.
    ///
    /// Returns an error if this `externref` GC reference has been unrooted (eg
    /// if you attempt to use a `Rooted<ExternRef>` after exiting the scope it
    /// was rooted within). See the documentation for
    /// [`Rooted<T>`][crate::Rooted] for more details.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn _foo() -> Result<()> {
    /// let mut store = Store::<()>::default();
    ///
    /// let externref = ExternRef::new::<usize>(&mut store, 0)?;
    ///
    /// // Access the `externref`'s host data.
    /// let data = externref.data_mut(&mut store)?;
    /// // Dowcast it to a `usize`.
    /// let data = data.downcast_mut::<usize>().ok_or_else(|| Error::msg("not a usize"))?;
    /// // We initialized to zero.
    /// assert_eq!(*data, 0);
    /// // And we can mutate the value!
    /// *data += 10;
    /// # Ok(())
    /// # }
    /// ```
    pub fn data_mut<'a, T>(
        &self,
        store: impl Into<StoreContextMut<'a, T>>,
    ) -> Result<&'a mut (dyn Any + Send + Sync)>
    where
        T: 'a,
    {
        let store = store.into().0;
        let gc_ref = self.inner.unchecked_try_gc_ref(store)?.unchecked_copy();
        let externref = gc_ref.as_externref_unchecked();
        Ok(store.gc_store_mut()?.externref_host_data_mut(externref))
    }

    /// Creates a new strongly-owned [`ExternRef`] from the raw value provided.
    ///
    /// This is intended to be used in conjunction with [`Func::new_unchecked`],
    /// [`Func::call_unchecked`], and [`ValRaw`] with its `externref` field.
    ///
    /// This function assumes that `raw` is an externref value which is
    /// currently rooted within the [`Store`].
    ///
    /// # Unsafety
    ///
    /// This function is particularly `unsafe` because `raw` not only must be a
    /// valid externref value produced prior by `to_raw` but it must also be
    /// correctly rooted within the store. When arguments are provided to a
    /// callback with [`Func::new_unchecked`], for example, or returned via
    /// [`Func::call_unchecked`], if a GC is performed within the store then
    /// floating externref values are not rooted and will be GC'd, meaning that
    /// this function will no longer be safe to call with the values cleaned up.
    /// This function must be invoked *before* possible GC operations can happen
    /// (such as calling wasm).
    ///
    /// When in doubt try to not use this. Instead use the safe Rust APIs of
    /// [`TypedFunc`] and friends.
    ///
    /// [`Func::call_unchecked`]: crate::Func::call_unchecked
    /// [`Func::new_unchecked`]: crate::Func::new_unchecked
    /// [`Store`]: crate::Store
    /// [`TypedFunc`]: crate::TypedFunc
    /// [`ValRaw`]: crate::ValRaw
    pub unsafe fn from_raw(mut store: impl AsContextMut, raw: u32) -> Option<Rooted<ExternRef>> {
        let mut store = AutoAssertNoGc::new(store.as_context_mut().0);
        let gc_ref = VMGcRef::from_raw_u32(raw)?;
        let gc_ref = store.unwrap_gc_store_mut().clone_gc_ref(&gc_ref);
        Some(Self::from_cloned_gc_ref(&mut store, gc_ref))
    }

    /// Converts this [`ExternRef`] to a raw value suitable to store within a
    /// [`ValRaw`].
    ///
    /// Returns an error if this `externref` has been unrooted.
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
        store.unwrap_gc_store_mut().expose_gc_ref_to_wasm(gc_ref);
        Ok(raw)
    }
}

unsafe impl WasmTy for Rooted<ExternRef> {
    // TODO: this should be `VMGcRef` but Cranelift currently doesn't support
    // using r32 types when targeting 64-bit platforms.
    type Abi = NonZeroU64;

    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Extern))
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.comes_from_same_store(store)
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &FuncType) -> Result<()> {
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
        ExternRef::from_cloned_gc_ref(store, gc_ref)
    }
}

unsafe impl WasmTy for Option<Rooted<ExternRef>> {
    type Abi = u64;

    #[inline]
    fn valtype() -> ValType {
        ValType::EXTERNREF
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.map_or(true, |x| x.comes_from_same_store(store))
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &FuncType) -> Result<()> {
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
            <Rooted<ExternRef> as WasmTy>::into_abi(x, store)?.get()
        } else {
            0
        })
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, store: &mut AutoAssertNoGc<'_>) -> Self {
        let gc_ref = VMGcRef::from_r64(abi).expect("valid r64")?;
        let gc_ref = store.unwrap_gc_store_mut().clone_gc_ref(&gc_ref);
        Some(ExternRef::from_cloned_gc_ref(store, gc_ref))
    }
}

unsafe impl WasmTy for ManuallyRooted<ExternRef> {
    type Abi = NonZeroU64;

    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Extern))
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.comes_from_same_store(store)
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &FuncType) -> Result<()> {
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
            let rooted = ExternRef::from_cloned_gc_ref(store, gc_ref);
            rooted
                ._to_manually_rooted(store)
                .expect("rooted is in scope")
        })
    }
}

unsafe impl WasmTy for Option<ManuallyRooted<ExternRef>> {
    type Abi = u64;

    #[inline]
    fn valtype() -> ValType {
        ValType::EXTERNREF
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.as_ref()
            .map_or(true, |x| x.comes_from_same_store(store))
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &FuncType) -> Result<()> {
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
            <ManuallyRooted<ExternRef> as WasmTy>::into_abi(x, store)?.get()
        } else {
            0
        })
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, store: &mut AutoAssertNoGc<'_>) -> Self {
        let gc_ref = VMGcRef::from_r64(abi).expect("valid r64")?;
        let gc_ref = store.unwrap_gc_store_mut().clone_gc_ref(&gc_ref);
        RootSet::with_lifo_scope(store, |store| {
            let rooted = ExternRef::from_cloned_gc_ref(store, gc_ref);
            Some(
                rooted
                    ._to_manually_rooted(store)
                    .expect("rooted is in scope"),
            )
        })
    }
}
