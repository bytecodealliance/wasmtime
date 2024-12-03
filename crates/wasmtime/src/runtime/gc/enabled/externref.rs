//! Implementation of `externref` in Wasmtime.

use super::{AnyRef, RootedGcRefImpl};
use crate::prelude::*;
use crate::runtime::vm::VMGcRef;
use crate::{
    store::{AutoAssertNoGc, StoreOpaque},
    AsContextMut, GcHeapOutOfMemory, GcRefImpl, GcRootIndex, HeapType, ManuallyRooted, RefType,
    Result, Rooted, StoreContext, StoreContextMut, ValRaw, ValType, WasmTy,
};
use core::any::Any;
use core::mem;
use core::mem::MaybeUninit;

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
/// Like all WebAssembly references, these are opaque and unforgeable to Wasm:
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
///             .ok_or_else(|| Error::msg("externref has no host data"))?
///             .downcast_ref::<Cow<str>>()
///             .ok_or_else(|| Error::msg("externref was not a string"))?
///             .clone()
///             .into_owned();
///         let b = b
///             .data(&caller)?
///             .ok_or_else(|| Error::msg("externref has no host data"))?
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
///     result
///         .data(&store)?
///         .expect("externref should have host data")
///         .downcast_ref::<Cow<str>>()
///         .expect("host data should be a `Cow<str>`"),
///     "Hello, World!"
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct ExternRef {
    pub(crate) inner: GcRootIndex,
}

unsafe impl GcRefImpl for ExternRef {
    #[allow(private_interfaces)]
    fn transmute_ref(index: &GcRootIndex) -> &Self {
        // Safety: `ExternRef` is a newtype of a `GcRootIndex`.
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

    /// Convert an `anyref` into an `externref`.
    ///
    /// This is equivalent to the `extern.convert_any` instruction in Wasm.
    ///
    /// You can get the underlying `anyref` again via the
    /// [`AnyRef::convert_extern`] method or the `any.convert_extern` Wasm
    /// instruction.
    ///
    /// The resulting `ExternRef` will not have any host data associated with
    /// it, so [`ExternRef::data`] and [`ExternRef::data_mut`] will both return
    /// `None`.
    ///
    /// Returns an error if the `anyref` GC reference has been unrooted (eg if
    /// you attempt to use a `Rooted<AnyRef>` after exiting the scope it was
    /// rooted within). See the documentation for [`Rooted<T>`][crate::Rooted]
    /// for more details.
    ///
    /// # Example
    ///
    /// ```
    /// use wasmtime::*;
    /// # fn foo() -> Result<()> {
    /// let engine = Engine::default();
    /// let mut store = Store::new(&engine, ());
    ///
    /// // Create an `anyref`.
    /// let i31 = I31::wrapping_u32(0x1234);
    /// let anyref = AnyRef::from_i31(&mut store, i31);
    ///
    /// // Convert that `anyref` into an `externref`.
    /// let externref = ExternRef::convert_any(&mut store, anyref)?;
    ///
    /// // This `externref` doesn't have any associated host data.
    /// assert!(externref.data(&store)?.is_none());
    ///
    /// // We can convert it back to an `anyref` and get its underlying `i31`
    /// // data.
    /// let anyref = AnyRef::convert_extern(&mut store, externref)?;
    /// assert_eq!(anyref.unwrap_i31(&store)?.get_u32(), 0x1234);
    /// # Ok(()) }
    /// # foo().unwrap();
    pub fn convert_any(
        mut context: impl AsContextMut,
        anyref: Rooted<AnyRef>,
    ) -> Result<Rooted<ExternRef>> {
        let mut store = AutoAssertNoGc::new(context.as_context_mut().0);
        Self::_convert_any(&mut store, anyref)
    }

    pub(crate) fn _convert_any(
        store: &mut AutoAssertNoGc<'_>,
        anyref: Rooted<AnyRef>,
    ) -> Result<Rooted<ExternRef>> {
        let gc_ref = anyref.try_clone_gc_ref(store)?;
        Ok(Self::from_cloned_gc_ref(store, gc_ref))
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
            gc_ref.is_extern_ref(&*store.unwrap_gc_store().gc_heap)
                || gc_ref.is_any_ref(&*store.unwrap_gc_store().gc_heap),
            "GC reference {gc_ref:#p} should be an externref or anyref"
        );
        Rooted::new(store, gc_ref)
    }

    /// Get a shared borrow of the underlying data for this `ExternRef`.
    ///
    /// Returns `None` if this is an `externref` wrapper of an `anyref` created
    /// by the `extern.convert_any` instruction or the
    /// [`ExternRef::convert_any`] method.
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
    /// let data = externref.data(&store)?.ok_or_else(|| Error::msg("no host data"))?;
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
    ) -> Result<Option<&'a (dyn Any + Send + Sync)>>
    where
        T: 'a,
    {
        let store = store.into().0;
        let gc_ref = self.inner.try_gc_ref(&store)?;
        let gc_store = store.gc_store()?;
        if let Some(externref) = gc_ref.as_externref(&*gc_store.gc_heap) {
            Ok(Some(gc_store.externref_host_data(externref)))
        } else {
            Ok(None)
        }
    }

    /// Get an exclusive borrow of the underlying data for this `ExternRef`.
    ///
    /// Returns `None` if this is an `externref` wrapper of an `anyref` created
    /// by the `extern.convert_any` instruction or the
    /// [`ExternRef::convert_any`] constructor.
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
    /// let data = externref.data_mut(&mut store)?.ok_or_else(|| Error::msg("no host data"))?;
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
    ) -> Result<Option<&'a mut (dyn Any + Send + Sync)>>
    where
        T: 'a,
    {
        let store = store.into().0;
        // NB: need to do an unchecked copy to release the borrow on the store
        // so that we can get the store's GC store. But importantly we cannot
        // trigger a GC while we are working with `gc_ref` here.
        let gc_ref = self.inner.try_gc_ref(store)?.unchecked_copy();
        let gc_store = store.gc_store_mut()?;
        if let Some(externref) = gc_ref.as_externref(&*gc_store.gc_heap) {
            Ok(Some(gc_store.externref_host_data_mut(externref)))
        } else {
            Ok(None)
        }
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
        Self::_from_raw(&mut store, raw)
    }

    // (Not actually memory unsafe since we have indexed GC heaps.)
    pub(crate) fn _from_raw(store: &mut AutoAssertNoGc, raw: u32) -> Option<Rooted<ExternRef>> {
        let gc_ref = VMGcRef::from_raw_u32(raw)?;
        let gc_ref = store.unwrap_gc_store_mut().clone_gc_ref(&gc_ref);
        Some(Self::from_cloned_gc_ref(store, gc_ref))
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
        self._to_raw(&mut store)
    }

    pub(crate) fn _to_raw(&self, store: &mut AutoAssertNoGc) -> Result<u32> {
        let gc_ref = self.inner.try_clone_gc_ref(store)?;
        let raw = gc_ref.as_raw_u32();
        store.unwrap_gc_store_mut().expose_gc_ref_to_wasm(gc_ref);
        Ok(raw)
    }
}

unsafe impl WasmTy for Rooted<ExternRef> {
    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Extern))
    }

    #[inline]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        self.comes_from_same_store(store)
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
        unreachable!()
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        self.wasm_ty_store(store, ptr, ValRaw::externref)
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        Self::wasm_ty_load(store, ptr.get_externref(), ExternRef::from_cloned_gc_ref)
    }
}

unsafe impl WasmTy for Option<Rooted<ExternRef>> {
    #[inline]
    fn valtype() -> ValType {
        ValType::EXTERNREF
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
    fn is_vmgcref_and_points_to_object(&self) -> bool {
        self.is_some()
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        <Rooted<ExternRef>>::wasm_ty_option_store(self, store, ptr, ValRaw::externref)
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        <Rooted<ExternRef>>::wasm_ty_option_load(
            store,
            ptr.get_externref(),
            ExternRef::from_cloned_gc_ref,
        )
    }
}

unsafe impl WasmTy for ManuallyRooted<ExternRef> {
    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Extern))
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
    fn is_vmgcref_and_points_to_object(&self) -> bool {
        true
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        self.wasm_ty_store(store, ptr, ValRaw::externref)
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        Self::wasm_ty_load(store, ptr.get_externref(), ExternRef::from_cloned_gc_ref)
    }
}

unsafe impl WasmTy for Option<ManuallyRooted<ExternRef>> {
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
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
        unreachable!()
    }

    #[inline]
    fn is_vmgcref_and_points_to_object(&self) -> bool {
        self.is_some()
    }

    fn store(self, store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        <ManuallyRooted<ExternRef>>::wasm_ty_option_store(self, store, ptr, ValRaw::externref)
    }

    unsafe fn load(store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        <ManuallyRooted<ExternRef>>::wasm_ty_option_load(
            store,
            ptr.get_externref(),
            ExternRef::from_cloned_gc_ref,
        )
    }
}
