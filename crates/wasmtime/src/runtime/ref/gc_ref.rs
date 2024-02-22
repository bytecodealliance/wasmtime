mod rooting;

use anyhow::anyhow;
pub use rooting::*;

use crate::{store::StoreOpaque, AsContext, AsContextMut, Result};
use std::any::Any;
use std::ffi::c_void;
use wasmtime_runtime::{VMExternRef, VMGcRef};

/// Upcast a `VMExternRef` into a `VMGcRef`.
pub(crate) fn extern_ref_into_gc_ref(e: VMExternRef) -> VMGcRef {
    unsafe { VMGcRef::from_ptr(e.into_raw()).unwrap() }
}

/// Downcast a `VMGcRef` into a `VMExternRef`.
///
/// # Safety
///
/// The given `gc_ref` must be a valid `VMExternRef`.
///
/// The resulting `VMExternRef` has non-atomic reference counting, and must only
/// be used within the context of a mutable store/GC heap.
pub(crate) unsafe fn extern_ref_from_gc_ref(gc_ref: VMGcRef) -> VMExternRef {
    VMExternRef::from_raw(gc_ref.as_non_null().as_ptr()).unwrap()
}

/// Clone a `VMExternRef` from a `VMGcRef`.
///
/// This requires mutable access to the store to ensure that the underlying
/// non-atomic reference counting is safe.
///
/// # Safety
///
/// The given `gc_ref` must be a valid `VMExternRef`.
///
/// The `gc_ref` must be associated with this store.
pub(crate) unsafe fn clone_extern_ref_from_gc_ref(
    _store: &mut StoreOpaque,
    gc_ref: VMGcRef,
) -> VMExternRef {
    VMExternRef::clone_from_raw(gc_ref.as_non_null().as_ptr()).unwrap()
}

unsafe fn extend_to_store_lifetime<'a, 'b, T>(_store: &'a impl AsContext, reference: &'b T) -> &'a T
where
    'a: 'b,
    T: ?Sized,
{
    std::mem::transmute(reference)
}

unsafe fn extend_to_store_lifetime_mut<'a, 'b, T>(
    _store: &'a mut impl AsContextMut,
    reference: &'b mut T,
) -> &'a mut T
where
    'a: 'b,
    T: ?Sized,
{
    std::mem::transmute(reference)
}

/// An opaque, GC-managed reference to some host data that can be passed to
/// WebAssembly.
///
/// The `ExternRef` type represents WebAssembly `externref` values. These are
/// opaque and unforgable to Wasm: they cannot be faked and Wasm can't, for
/// example, cast the integer `0x12345678` into a reference, pretend it is a
/// valid `externref`, and trick the host into dereferencing it and segfaulting
/// or worse. Wasm can't do anything with the `externref`s other than put them
/// in tables, globals, and locals or pass them to other functions.
///
/// You can use `ExternRef` to give access to host objects and control the
/// operations that Wasm can perform on them via what functions you allow Wasm
/// to import.
///
/// Note that you can also use `Rooted<ExternRef>` as a type parameter with
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
///     |caller: Caller<'_, ()>| -> Rooted<ExternRef> { ExternRef::new(caller, Cow::from("")) },
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
///         Ok(ExternRef::new(&mut caller, s))
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
/// let hello = ExternRef::new(&mut store, Cow::from("Hello, "));
/// let world = ExternRef::new(&mut store, Cow::from("World!"));
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
#[derive(Debug)]
#[repr(transparent)]
pub struct ExternRef {
    inner: GcRootIndex,
}

unsafe impl GcRefImpl for ExternRef {}

impl ExternRef {
    /// Creates a new instance of `ExternRef` wrapping the given value.
    ///
    /// The resulting value is automatically unrooted when the given `context`'s
    /// scope is exited. See [`Rooted<T>`][crate::Rooted]'s documentation for
    /// more details.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn _foo() -> Result<()> {
    /// let mut store = Store::<()>::default();
    ///
    /// // Create an `externref` wrapping a `str`.
    /// let externref = ExternRef::new(&mut store, "hello!");
    /// # Ok(())
    /// # }
    /// ```
    pub fn new<T>(mut context: impl AsContextMut, value: T) -> Rooted<ExternRef>
    where
        T: 'static + Any + Send + Sync,
    {
        // Safety: We proviode `VMExternRef`'s invariants via the way that
        // `ExternRef` methods take `impl AsContext[Mut]` methods.
        let inner = unsafe { VMExternRef::new(value) };
        Self::from_vm_extern_ref(context.as_context_mut().0, inner)
    }

    /// Creates a new, manually-rooted instance of `ExternRef` wrapping the
    /// given value.
    ///
    /// The resulting value must be manually unrooted, or else it will leak for
    /// the entire duration of the store's lifetime. See
    /// [`ManuallyRooted<T>`][crate::ManuallyRooted]'s documentation for more
    /// details.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn _foo() -> Result<()> {
    /// let mut store = Store::<()>::default();
    ///
    /// let externref = ExternRef::new_manually_rooted(&mut store, "hello!");
    ///
    /// // Use `externref` a bunch...
    ///
    /// // Don't forget to unroot the `externref` when done using it.
    /// externref.unroot(&mut store);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_manually_rooted<T>(
        mut store: impl AsContextMut,
        value: T,
    ) -> ManuallyRooted<ExternRef>
    where
        T: 'static + Any + Send + Sync,
    {
        // Safety: We proviode `VMExternRef`'s invariants via the way that
        // `ExternRef` methods take `impl AsContext[Mut]` methods.
        let inner = unsafe { VMExternRef::new(value) };
        let inner = extern_ref_into_gc_ref(inner);

        // Safety: `inner` is a GC reference pointing to an `externref` GC
        // object.
        unsafe { ManuallyRooted::new(store.as_context_mut().0, inner) }
    }

    pub(crate) fn from_vm_extern_ref(store: &mut StoreOpaque, inner: VMExternRef) -> Rooted<Self> {
        let inner = extern_ref_into_gc_ref(inner);
        // Safety: `inner` is a GC reference pointing to an `externref` GC
        // object.
        unsafe { Rooted::new(store, inner) }
    }

    pub(crate) fn to_vm_extern_ref(&self, store: &mut StoreOpaque) -> Option<VMExternRef> {
        let gc_ref = self.inner.get_gc_ref(store)?;
        // Safety: Our underlying `gc_ref` is always pointing to an `externref`.
        Some(unsafe { clone_extern_ref_from_gc_ref(store, gc_ref) })
    }

    pub(crate) fn try_to_vm_extern_ref(&self, store: &mut StoreOpaque) -> Result<VMExternRef> {
        self.to_vm_extern_ref(store)
            .ok_or_else(|| anyhow!("attempted to use an `externref` that was unrooted"))
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
    /// let externref = ExternRef::new(&mut store, "hello");
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
    pub fn data<'a>(&self, store: &'a impl AsContext) -> Result<&'a (dyn Any + Send + Sync)> {
        let gc_ref = self.inner.try_gc_ref(store.as_context().0)?.as_non_null();
        unsafe {
            let inner = VMExternRef::ref_from_raw(&gc_ref);
            Ok(extend_to_store_lifetime(store, inner.data()))
        }
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
    /// let externref = ExternRef::new::<usize>(&mut store, 0);
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
    pub fn data_mut<'a>(
        &self,
        store: &'a mut impl AsContextMut,
    ) -> Result<&'a mut (dyn Any + Send + Sync)> {
        let mut gc_ref = self.inner.try_gc_ref(store.as_context().0)?.as_non_null();
        unsafe {
            let inner = VMExternRef::ref_mut_from_raw(&mut gc_ref);
            Ok(extend_to_store_lifetime_mut(store, inner.data_mut()))
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
    pub unsafe fn from_raw(
        mut store: impl AsContextMut,
        raw: *mut c_void,
    ) -> Option<Rooted<ExternRef>> {
        let raw = raw.cast::<u8>();
        let inner = VMExternRef::clone_from_raw(raw)?;
        Some(Self::from_vm_extern_ref(store.as_context_mut().0, inner))
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
    pub unsafe fn to_raw(&self, mut store: impl AsContextMut) -> Result<*mut c_void> {
        let store = store.as_context_mut().0;
        let gc_ref = self.inner.try_gc_ref(store)?;
        let inner = clone_extern_ref_from_gc_ref(store, gc_ref);
        let raw = inner.as_raw();
        store.insert_vmexternref_without_gc(inner);
        Ok(raw.cast())
    }
}
