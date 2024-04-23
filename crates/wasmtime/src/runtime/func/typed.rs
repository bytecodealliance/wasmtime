use super::{invoke_wasm_and_catch_traps, HostAbi};
use crate::store::{AutoAssertNoGc, StoreOpaque};
use crate::{
    AsContext, AsContextMut, Engine, Func, FuncType, HeapType, NoFunc, RefType, StoreContextMut,
    ValRaw, ValType,
};
use anyhow::{bail, Context, Result};
use std::marker;
use std::mem::{self, MaybeUninit};
use std::num::NonZeroUsize;
use std::os::raw::c_void;
use std::ptr::{self, NonNull};
use wasmtime_environ::VMSharedTypeIndex;
use wasmtime_runtime::{VMContext, VMFuncRef, VMNativeCallFunction, VMOpaqueContext};

/// A statically typed WebAssembly function.
///
/// Values of this type represent statically type-checked WebAssembly functions.
/// The function within a [`TypedFunc`] is statically known to have `Params` as its
/// parameters and `Results` as its results.
///
/// This structure is created via [`Func::typed`] or [`TypedFunc::new_unchecked`].
/// For more documentation about this see those methods.
pub struct TypedFunc<Params, Results> {
    _a: marker::PhantomData<fn(Params) -> Results>,
    ty: FuncType,
    func: Func,
}

impl<Params, Results> Clone for TypedFunc<Params, Results> {
    fn clone(&self) -> TypedFunc<Params, Results> {
        Self {
            _a: marker::PhantomData,
            ty: self.ty.clone(),
            func: self.func,
        }
    }
}

impl<Params, Results> TypedFunc<Params, Results>
where
    Params: WasmParams,
    Results: WasmResults,
{
    /// An unchecked version of [`Func::typed`] which does not perform a
    /// typecheck and simply assumes that the type declared here matches the
    /// type of this function.
    ///
    /// The semantics of this function are the same as [`Func::typed`] except
    /// that no error is returned because no typechecking is done.
    ///
    /// # Unsafety
    ///
    /// This function only safe to call if `typed` would otherwise return `Ok`
    /// for the same `Params` and `Results` specified. If `typed` would return
    /// an error then the returned `TypedFunc` is memory unsafe to invoke.
    pub unsafe fn new_unchecked(store: impl AsContext, func: Func) -> TypedFunc<Params, Results> {
        let store = store.as_context().0;
        Self::_new_unchecked(store, func)
    }

    pub(crate) unsafe fn _new_unchecked(
        store: &StoreOpaque,
        func: Func,
    ) -> TypedFunc<Params, Results> {
        let ty = func.load_ty(store);
        TypedFunc {
            _a: marker::PhantomData,
            ty,
            func,
        }
    }

    /// Returns the underlying [`Func`] that this is wrapping, losing the static
    /// type information in the process.
    pub fn func(&self) -> &Func {
        &self.func
    }

    /// Invokes this WebAssembly function with the specified parameters.
    ///
    /// Returns either the results of the call, or a [`Trap`] if one happened.
    ///
    /// For more information, see the [`Func::typed`] and [`Func::call`]
    /// documentation.
    ///
    /// # Errors
    ///
    /// For more information on errors see the documentation on [`Func::call`].
    ///
    /// # Panics
    ///
    /// This function will panic if it is called when the underlying [`Func`] is
    /// connected to an asynchronous store.
    ///
    /// [`Trap`]: crate::Trap
    pub fn call(&self, mut store: impl AsContextMut, params: Params) -> Result<Results> {
        let mut store = store.as_context_mut();
        assert!(
            !store.0.async_support(),
            "must use `call_async` with async stores"
        );
        if Self::need_gc_before_call_raw(store.0, &params) {
            store.0.gc();
        }
        let func = self.func.vm_func_ref(store.0);
        unsafe { Self::call_raw(&mut store, &self.ty, func, params) }
    }

    /// Invokes this WebAssembly function with the specified parameters.
    ///
    /// Returns either the results of the call, or a [`Trap`] if one happened.
    ///
    /// For more information, see the [`Func::typed`] and [`Func::call_async`]
    /// documentation.
    ///
    /// # Errors
    ///
    /// For more information on errors see the documentation on [`Func::call`].
    ///
    /// # Panics
    ///
    /// This function will panic if it is called when the underlying [`Func`] is
    /// connected to a synchronous store.
    ///
    /// [`Trap`]: crate::Trap
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub async fn call_async<T>(
        &self,
        mut store: impl AsContextMut<Data = T>,
        params: Params,
    ) -> Result<Results>
    where
        T: Send,
    {
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "must use `call` with non-async stores"
        );
        if Self::need_gc_before_call_raw(store.0, &params) {
            store.0.gc_async().await;
        }
        store
            .on_fiber(|store| {
                let func = self.func.vm_func_ref(store.0);
                unsafe { Self::call_raw(store, &self.ty, func, params) }
            })
            .await?
    }

    #[inline]
    pub(crate) fn need_gc_before_call_raw(_store: &StoreOpaque, _params: &Params) -> bool {
        #[cfg(feature = "gc")]
        {
            // See the comment in `Func::call_impl_check_args`.
            let num_gc_refs = _params.non_i31_gc_refs_count();
            if let Some(num_gc_refs) = NonZeroUsize::new(num_gc_refs) {
                return _store
                    .unwrap_gc_store()
                    .gc_heap
                    .need_gc_before_entering_wasm(num_gc_refs);
            }
        }

        false
    }

    /// Do a raw call of a typed function.
    ///
    /// # Safety
    ///
    /// `func` must be of the given type.
    ///
    /// If `Self::need_gc_before_call_raw`, then the caller must have done a GC
    /// just before calling this method.
    pub(crate) unsafe fn call_raw<T>(
        store: &mut StoreContextMut<'_, T>,
        ty: &FuncType,
        func: ptr::NonNull<VMFuncRef>,
        params: Params,
    ) -> Result<Results> {
        // double-check that params/results match for this function's type in
        // debug mode.
        if cfg!(debug_assertions) {
            Self::debug_typecheck(store.0, func.as_ref().type_index);
        }

        // Validate that all runtime values flowing into this store indeed
        // belong within this store, otherwise it would be unsafe for store
        // values to cross each other.

        let params = {
            let mut store = AutoAssertNoGc::new(store.0);
            params.into_abi(&mut store, ty)?
        };

        // Try to capture only a single variable (a tuple) in the closure below.
        // This means the size of the closure is one pointer and is much more
        // efficient to move in memory. This closure is actually invoked on the
        // other side of a C++ shim, so it can never be inlined enough to make
        // the memory go away, so the size matters here for performance.
        let mut captures = (func, MaybeUninit::uninit(), params, false);

        let result = invoke_wasm_and_catch_traps(store, |caller| {
            let (func_ref, ret, params, returned) = &mut captures;
            let func_ref = func_ref.as_ref();
            let result =
                Params::invoke::<Results>(func_ref.native_call, func_ref.vmctx, caller, *params);
            ptr::write(ret.as_mut_ptr(), result);
            *returned = true
        });

        let (_, ret, _, returned) = captures;
        debug_assert_eq!(result.is_ok(), returned);
        result?;

        let mut store = AutoAssertNoGc::new(store.0);
        Ok(Results::from_abi(&mut store, ret.assume_init()))
    }

    /// Purely a debug-mode assertion, not actually used in release builds.
    fn debug_typecheck(store: &StoreOpaque, func: VMSharedTypeIndex) {
        let ty = FuncType::from_shared_type_index(store.engine(), func);
        Params::typecheck(store.engine(), ty.params(), TypeCheckPosition::Param)
            .expect("params should match");
        Results::typecheck(store.engine(), ty.results(), TypeCheckPosition::Result)
            .expect("results should match");
    }
}

#[doc(hidden)]
#[derive(Copy, Clone)]
pub enum TypeCheckPosition {
    Param,
    Result,
}

/// A trait implemented for types which can be arguments and results for
/// closures passed to [`Func::wrap`] as well as parameters to [`Func::typed`].
///
/// This trait should not be implemented by user types. This trait may change at
/// any time internally. The types which implement this trait, however, are
/// stable over time.
///
/// For more information see [`Func::wrap`] and [`Func::typed`]
pub unsafe trait WasmTy: Send {
    // The raw ABI type that values of this type can be converted to and passed
    // to Wasm, or given from Wasm and converted back from.
    #[doc(hidden)]
    type Abi: 'static + Copy;

    // Do a "static" (aka at time of `func.typed::<P, R>()`) ahead-of-time type
    // check for this type at the given position. You probably don't need to
    // override this trait method.
    #[doc(hidden)]
    #[inline]
    fn typecheck(engine: &Engine, actual: ValType, position: TypeCheckPosition) -> Result<()> {
        let expected = Self::valtype();
        debug_assert!(expected.comes_from_same_engine(engine));
        debug_assert!(actual.comes_from_same_engine(engine));
        match position {
            // The caller is expecting to receive a `T` and the callee is
            // actually returning a `U`, so ensure that `U <: T`.
            TypeCheckPosition::Result => actual.ensure_matches(engine, &expected),
            // The caller is expecting to pass a `T` and the callee is expecting
            // to receive a `U`, so ensure that `T <: U`.
            TypeCheckPosition::Param => match (expected.as_ref(), actual.as_ref()) {
                // ... except that this technically-correct check would overly
                // restrict the usefulness of our typed function APIs for the
                // specific case of concrete reference types. Let's work through
                // an example.
                //
                // Consider functions that take a `(ref param $some_func_type)`
                // parameter:
                //
                // * We cannot have a static `wasmtime::SomeFuncTypeRef` type
                //   that implements `WasmTy` specifically for `(ref null
                //   $some_func_type)` because Wasm modules, and their types,
                //   are loaded dynamically at runtime.
                //
                // * Therefore the embedder's only option for `T <: (ref null
                //   $some_func_type)` is `T = (ref null nofunc)` aka
                //   `Option<wasmtime::NoFunc>`.
                //
                // * But that static type means they can *only* pass in the null
                //   function reference as an argument to the typed function.
                //   This is way too restrictive! For ergonomics, we want them
                //   to be able to pass in a `wasmtime::Func` whose type is
                //   `$some_func_type`!
                //
                // To lift this constraint and enable better ergonomics for
                // embedders, we allow `top(T) <: top(U)` -- i.e. they are part
                // of the same type hierarchy and a dynamic cast could possibly
                // succeed -- for the specific case of concrete heap type
                // parameters, and fall back to dynamic type checks on the
                // arguments passed to each invocation, as necessary.
                (Some(expected_ref), Some(actual_ref)) if actual_ref.heap_type().is_concrete() => {
                    expected_ref
                        .heap_type()
                        .top()
                        .ensure_matches(engine, &actual_ref.heap_type().top())
                }
                _ => expected.ensure_matches(engine, &actual),
            },
        }
    }

    // The value type that this Type represents.
    #[doc(hidden)]
    fn valtype() -> ValType;

    // Dynamic checks that this value is being used with the correct store
    // context.
    #[doc(hidden)]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool;

    // Dynamic checks that `self <: actual` for concrete type arguments. See the
    // comment above in `WasmTy::typecheck`.
    //
    // Only ever called for concrete reference type arguments, so any type which
    // is not in a type hierarchy with concrete reference types can implement
    // this with `unreachable!()`.
    #[doc(hidden)]
    fn dynamic_concrete_type_check(
        &self,
        store: &StoreOpaque,
        nullable: bool,
        actual: &HeapType,
    ) -> Result<()>;

    // Is this an externref?
    #[doc(hidden)]
    fn is_non_i31_gc_ref(&self) -> bool;

    // Construct a `Self::Abi` from the given `ValRaw`.
    #[doc(hidden)]
    unsafe fn abi_from_raw(raw: *mut ValRaw) -> Self::Abi;

    // Stuff our given `Self::Abi` into a `ValRaw`.
    #[doc(hidden)]
    unsafe fn abi_into_raw(abi: Self::Abi, raw: *mut ValRaw);

    // Convert `self` into `Self::Abi`.
    //
    // NB: We _must not_ trigger a GC when passing refs from host code into Wasm
    // (e.g. returned from a host function or passed as arguments to a Wasm
    // function). After insertion into the activations table, the reference is
    // no longer rooted. If multiple references are being sent from the host
    // into Wasm and we allowed GCs during insertion, then the following events
    // could happen:
    //
    // * Reference A is inserted into the activations table. This does not
    //   trigger a GC, but does fill the table to capacity.
    //
    // * The caller's reference to A is removed. Now the only reference to A is
    //   from the activations table.
    //
    // * Reference B is inserted into the activations table. Because the table
    //   is at capacity, a GC is triggered.
    //
    // * A is reclaimed because the only reference keeping it alive was the
    //   activation table's reference (it isn't inside any Wasm frames on the
    //   stack yet, so stack scanning and stack maps don't increment its
    //   reference count).
    //
    // * We transfer control to Wasm, giving it A and B. Wasm uses A. That's a
    //   use-after-free bug.
    //
    // In conclusion, to prevent uses-after-free bugs, we cannot GC while
    // converting types into their raw ABI forms.
    #[doc(hidden)]
    fn into_abi(self, store: &mut AutoAssertNoGc<'_>) -> Result<Self::Abi>;

    // Convert back from `Self::Abi` into `Self`.
    #[doc(hidden)]
    unsafe fn from_abi(abi: Self::Abi, store: &mut AutoAssertNoGc<'_>) -> Self;
}

macro_rules! integers {
    ($($primitive:ident/$get_primitive:ident => $ty:ident)*) => ($(
        unsafe impl WasmTy for $primitive {
            type Abi = $primitive;
            #[inline]
            fn valtype() -> ValType {
                ValType::$ty
            }
            #[inline]
            fn compatible_with_store(&self, _: &StoreOpaque) -> bool {
                true
            }
            #[inline]
            fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
                unreachable!()
            }
            #[inline]
            fn is_non_i31_gc_ref(&self) -> bool {
                false
            }
            #[inline]
            unsafe fn abi_from_raw(raw: *mut ValRaw) -> $primitive {
                (*raw).$get_primitive()
            }
            #[inline]
            unsafe fn abi_into_raw(abi: $primitive, raw: *mut ValRaw) {
                *raw = ValRaw::$primitive(abi);
            }
            #[inline]
            fn into_abi(self, _store: &mut AutoAssertNoGc<'_>) -> Result<Self::Abi>
            {
                Ok(self)
            }
            #[inline]
            unsafe fn from_abi(abi: Self::Abi, _store: &mut AutoAssertNoGc<'_>) -> Self {
                abi
            }
        }
    )*)
}

integers! {
    i32/get_i32 => I32
    i64/get_i64 => I64
    u32/get_u32 => I32
    u64/get_u64 => I64
}

macro_rules! floats {
    ($($float:ident/$int:ident/$get_float:ident => $ty:ident)*) => ($(
        unsafe impl WasmTy for $float {
            type Abi = $float;
            #[inline]
            fn valtype() -> ValType {
                ValType::$ty
            }
            #[inline]
            fn compatible_with_store(&self, _: &StoreOpaque) -> bool {
                true
            }
            #[inline]
            fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
                unreachable!()
            }
            #[inline]
            fn is_non_i31_gc_ref(&self) -> bool {
                false
            }
            #[inline]
            unsafe fn abi_from_raw(raw: *mut ValRaw) -> $float {
                $float::from_bits((*raw).$get_float())
            }
            #[inline]
            unsafe fn abi_into_raw(abi: $float, raw: *mut ValRaw) {
                *raw = ValRaw::$float(abi.to_bits());
            }
            #[inline]
            fn into_abi(self, _store: &mut AutoAssertNoGc<'_>) -> Result<Self::Abi>
            {
                Ok(self)
            }
            #[inline]
            unsafe fn from_abi(abi: Self::Abi, _store: &mut AutoAssertNoGc<'_>) -> Self {
                abi
            }
        }
    )*)
}

floats! {
    f32/u32/get_f32 => F32
    f64/u64/get_f64 => F64
}

unsafe impl WasmTy for NoFunc {
    type Abi = NoFunc;

    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::NoFunc))
    }

    #[inline]
    fn compatible_with_store(&self, _store: &StoreOpaque) -> bool {
        match self._inner {}
    }

    #[inline]
    fn dynamic_concrete_type_check(&self, _: &StoreOpaque, _: bool, _: &HeapType) -> Result<()> {
        match self._inner {}
    }

    #[inline]
    fn is_non_i31_gc_ref(&self) -> bool {
        match self._inner {}
    }

    #[inline]
    unsafe fn abi_from_raw(_raw: *mut ValRaw) -> Self::Abi {
        unreachable!("NoFunc is uninhabited")
    }

    #[inline]
    unsafe fn abi_into_raw(_abi: Self::Abi, _raw: *mut ValRaw) {
        unreachable!("NoFunc is uninhabited")
    }

    #[inline]
    fn into_abi(self, _store: &mut AutoAssertNoGc<'_>) -> Result<Self::Abi> {
        unreachable!("NoFunc is uninhabited")
    }

    #[inline]
    unsafe fn from_abi(_abi: Self::Abi, _store: &mut AutoAssertNoGc<'_>) -> Self {
        unreachable!("NoFunc is uninhabited")
    }
}

unsafe impl WasmTy for Option<NoFunc> {
    type Abi = *mut NoFunc;

    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(true, HeapType::NoFunc))
    }

    #[inline]
    fn compatible_with_store(&self, _store: &StoreOpaque) -> bool {
        true
    }

    #[inline]
    fn dynamic_concrete_type_check(
        &self,
        _: &StoreOpaque,
        nullable: bool,
        ty: &HeapType,
    ) -> Result<()> {
        if nullable {
            // `(ref null nofunc) <: (ref null $f)` for all function types `$f`.
            Ok(())
        } else {
            bail!("argument type mismatch: expected non-nullable (ref {ty}), found null reference")
        }
    }

    #[inline]
    fn is_non_i31_gc_ref(&self) -> bool {
        false
    }

    #[inline]
    unsafe fn abi_from_raw(_raw: *mut ValRaw) -> Self::Abi {
        ptr::null_mut()
    }

    #[inline]
    unsafe fn abi_into_raw(_abi: Self::Abi, raw: *mut ValRaw) {
        *raw = ValRaw::funcref(ptr::null_mut());
    }

    #[inline]
    fn into_abi(self, _store: &mut AutoAssertNoGc<'_>) -> Result<Self::Abi> {
        Ok(ptr::null_mut())
    }

    #[inline]
    unsafe fn from_abi(_abi: Self::Abi, _store: &mut AutoAssertNoGc<'_>) -> Self {
        None
    }
}

unsafe impl WasmTy for Func {
    type Abi = NonNull<wasmtime_runtime::VMFuncRef>;

    #[inline]
    fn valtype() -> ValType {
        ValType::Ref(RefType::new(false, HeapType::Func))
    }

    #[inline]
    fn compatible_with_store<'a>(&self, store: &StoreOpaque) -> bool {
        store.store_data().contains(self.0)
    }

    #[inline]
    fn dynamic_concrete_type_check(
        &self,
        store: &StoreOpaque,
        _nullable: bool,
        expected: &HeapType,
    ) -> Result<()> {
        let expected = expected.unwrap_concrete_func();
        self.ensure_matches_ty(store, expected)
            .context("argument type mismatch for reference to concrete type")
    }

    #[inline]
    fn is_non_i31_gc_ref(&self) -> bool {
        false
    }

    #[inline]
    unsafe fn abi_from_raw(raw: *mut ValRaw) -> Self::Abi {
        let p = (*raw).get_funcref();
        debug_assert!(!p.is_null());
        NonNull::new_unchecked(p.cast::<wasmtime_runtime::VMFuncRef>())
    }

    #[inline]
    unsafe fn abi_into_raw(abi: Self::Abi, raw: *mut ValRaw) {
        *raw = ValRaw::funcref(abi.cast::<c_void>().as_ptr());
    }

    #[inline]
    fn into_abi(self, store: &mut AutoAssertNoGc<'_>) -> Result<Self::Abi> {
        Ok(self.vm_func_ref(store))
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, store: &mut AutoAssertNoGc<'_>) -> Self {
        Func::from_vm_func_ref(store, abi.as_ptr()).unwrap()
    }
}

unsafe impl WasmTy for Option<Func> {
    type Abi = *mut wasmtime_runtime::VMFuncRef;

    #[inline]
    fn valtype() -> ValType {
        ValType::FUNCREF
    }

    #[inline]
    fn compatible_with_store<'a>(&self, store: &StoreOpaque) -> bool {
        if let Some(f) = self {
            store.store_data().contains(f.0)
        } else {
            true
        }
    }

    fn dynamic_concrete_type_check(
        &self,
        store: &StoreOpaque,
        nullable: bool,
        expected: &HeapType,
    ) -> Result<()> {
        if let Some(f) = self {
            let expected = expected.unwrap_concrete_func();
            f.ensure_matches_ty(store, expected)
                .context("argument type mismatch for reference to concrete type")
        } else if nullable {
            Ok(())
        } else {
            bail!("argument type mismatch: expected non-nullable (ref {expected}), found null reference")
        }
    }

    #[inline]
    fn is_non_i31_gc_ref(&self) -> bool {
        false
    }

    #[inline]
    unsafe fn abi_from_raw(raw: *mut ValRaw) -> Self::Abi {
        (*raw).get_funcref() as Self::Abi
    }

    #[inline]
    unsafe fn abi_into_raw(abi: Self::Abi, raw: *mut ValRaw) {
        *raw = ValRaw::funcref(abi.cast());
    }

    #[inline]
    fn into_abi(self, store: &mut AutoAssertNoGc<'_>) -> Result<Self::Abi> {
        Ok(if let Some(f) = self {
            f.vm_func_ref(store).as_ptr()
        } else {
            ptr::null_mut()
        })
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, store: &mut AutoAssertNoGc<'_>) -> Self {
        Func::from_vm_func_ref(store, abi)
    }
}

/// A trait used for [`Func::typed`] and with [`TypedFunc`] to represent the set of
/// parameters for wasm functions.
///
/// This is implemented for bare types that can be passed to wasm as well as
/// tuples of those types.
pub unsafe trait WasmParams: Send {
    #[doc(hidden)]
    type Abi: Copy;

    #[doc(hidden)]
    fn typecheck(
        engine: &Engine,
        params: impl ExactSizeIterator<Item = crate::ValType>,
        position: TypeCheckPosition,
    ) -> Result<()>;

    #[doc(hidden)]
    fn non_i31_gc_refs_count(&self) -> usize;

    #[doc(hidden)]
    fn into_abi(self, store: &mut AutoAssertNoGc<'_>, func_ty: &FuncType) -> Result<Self::Abi>;

    #[doc(hidden)]
    unsafe fn invoke<R: WasmResults>(
        func: NonNull<VMNativeCallFunction>,
        vmctx1: *mut VMOpaqueContext,
        vmctx2: *mut VMContext,
        abi: Self::Abi,
    ) -> R::ResultAbi;
}

// Forward an impl from `T` to `(T,)` for convenience if there's only one
// parameter.
unsafe impl<T> WasmParams for T
where
    T: WasmTy,
{
    type Abi = <(T,) as WasmParams>::Abi;

    fn typecheck(
        engine: &Engine,
        params: impl ExactSizeIterator<Item = crate::ValType>,
        position: TypeCheckPosition,
    ) -> Result<()> {
        <(T,) as WasmParams>::typecheck(engine, params, position)
    }

    #[inline]
    fn non_i31_gc_refs_count(&self) -> usize {
        T::is_non_i31_gc_ref(self) as usize
    }

    #[inline]
    fn into_abi(self, store: &mut AutoAssertNoGc<'_>, func_ty: &FuncType) -> Result<Self::Abi> {
        <(T,) as WasmParams>::into_abi((self,), store, func_ty)
    }

    unsafe fn invoke<R: WasmResults>(
        func: NonNull<VMNativeCallFunction>,
        vmctx1: *mut VMOpaqueContext,
        vmctx2: *mut VMContext,
        abi: Self::Abi,
    ) -> R::ResultAbi {
        <(T,) as WasmParams>::invoke::<R>(func, vmctx1, vmctx2, abi)
    }
}

macro_rules! impl_wasm_params {
    ($n:tt $($t:ident)*) => {
        #[allow(non_snake_case)]
        unsafe impl<$($t: WasmTy,)*> WasmParams for ($($t,)*) {
            type Abi = ($($t::Abi,)*);

            fn typecheck(
                _engine: &Engine,
                mut params: impl ExactSizeIterator<Item = crate::ValType>,
                _position: TypeCheckPosition,
            ) -> Result<()> {
                let mut _n = 0;

                $(
                    match params.next() {
                        Some(t) => {
                            _n += 1;
                            $t::typecheck(_engine, t, _position)?
                        },
                        None => bail!("expected {} types, found {}", $n, params.len() + _n),
                    }
                )*

                match params.next() {
                    None => Ok(()),
                    Some(_) => {
                        _n += 1;
                        bail!("expected {} types, found {}", $n, params.len() + _n)
                    },
                }
            }

            #[inline]
            fn non_i31_gc_refs_count(&self) -> usize {
                let ($(ref $t,)*) = self;
                0 $(
                    + $t.is_non_i31_gc_ref() as usize
                )*
            }


            #[inline]
            fn into_abi(
                self,
                _store: &mut AutoAssertNoGc<'_>,
                _func_ty: &FuncType,
            ) -> Result<Self::Abi> {
                let ($($t,)*) = self;

                let mut _i = 0;
                $(
                    if !$t.compatible_with_store(_store) {
                        bail!("attempt to pass cross-`Store` value to Wasm as function argument");
                    }

                    if $t::valtype().is_ref() {
                        let param_ty = _func_ty.param(_i).unwrap();
                        let ref_ty = param_ty.unwrap_ref();
                        let heap_ty = ref_ty.heap_type();
                        if heap_ty.is_concrete() {
                            $t.dynamic_concrete_type_check(_store, ref_ty.is_nullable(), heap_ty)?;
                        }
                    }

                    let $t = $t.into_abi(_store)?;

                    _i += 1;
                )*
                Ok(($($t,)*))
            }

            unsafe fn invoke<R: WasmResults>(
                func: NonNull<VMNativeCallFunction>,
                vmctx1: *mut VMOpaqueContext,
                vmctx2: *mut VMContext,
                abi: Self::Abi,
            ) -> R::ResultAbi {
                let fnptr = mem::transmute::<
                    NonNull<VMNativeCallFunction>,
                    unsafe extern "C" fn(
                        *mut VMOpaqueContext,
                        *mut VMContext,
                        $($t::Abi,)*
                        <R::ResultAbi as HostAbi>::Retptr,
                    ) -> <R::ResultAbi as HostAbi>::Abi,
                    >(func);
                let ($($t,)*) = abi;
                // Use the `call` function to acquire a `retptr` which we'll
                // forward to the native function. Once we have it we also
                // convert all our arguments to abi arguments to go to the raw
                // function.
                //
                // Upon returning `R::call` will convert all the returns back
                // into `R`.
                <R::ResultAbi as HostAbi>::call(|retptr| {
                    fnptr(vmctx1, vmctx2, $($t,)* retptr)
                })
            }
        }
    };
}

for_each_function_signature!(impl_wasm_params);

/// A trait used for [`Func::typed`] and with [`TypedFunc`] to represent the set of
/// results for wasm functions.
pub unsafe trait WasmResults: WasmParams {
    #[doc(hidden)]
    type ResultAbi: HostAbi;

    #[doc(hidden)]
    unsafe fn from_abi(store: &mut AutoAssertNoGc<'_>, abi: Self::ResultAbi) -> Self;
}

// Forwards from a bare type `T` to the 1-tuple type `(T,)`
unsafe impl<T: WasmTy> WasmResults for T
where
    (T::Abi,): HostAbi,
{
    type ResultAbi = <(T,) as WasmResults>::ResultAbi;

    unsafe fn from_abi(store: &mut AutoAssertNoGc<'_>, abi: Self::ResultAbi) -> Self {
        <(T,) as WasmResults>::from_abi(store, abi).0
    }
}

macro_rules! impl_wasm_results {
    ($n:tt $($t:ident)*) => {
        #[allow(non_snake_case, unused_variables)]
        unsafe impl<$($t: WasmTy,)*> WasmResults for ($($t,)*)
            where ($($t::Abi,)*): HostAbi
        {
            type ResultAbi = ($($t::Abi,)*);

            #[inline]
            unsafe fn from_abi(store: &mut AutoAssertNoGc<'_>, abi: Self::ResultAbi) -> Self {
                let ($($t,)*) = abi;
                ($($t::from_abi($t, store),)*)
            }
        }
    };
}

for_each_function_signature!(impl_wasm_results);
