use super::{invoke_wasm_and_catch_traps, HostAbi};
use crate::store::{AutoAssertNoGc, StoreOpaque};
use crate::{AsContextMut, ExternRef, Func, FuncType, StoreContextMut, Trap, ValRaw, ValType};
use anyhow::{bail, Result};
use std::marker;
use std::mem::{self, MaybeUninit};
use std::ptr;
use wasmtime_runtime::{VMCallerCheckedAnyfunc, VMContext, VMFunctionBody, VMSharedSignatureIndex};

/// A statically typed WebAssembly function.
///
/// Values of this type represent statically type-checked WebAssembly functions.
/// The function within a [`TypedFunc`] is statically known to have `Params` as its
/// parameters and `Results` as its results.
///
/// This structure is created via [`Func::typed`] or [`TypedFunc::new_unchecked`].
/// For more documentation about this see those methods.
#[repr(transparent)] // here for the C API
pub struct TypedFunc<Params, Results> {
    _a: marker::PhantomData<fn(Params) -> Results>,
    func: Func,
}

impl<Params, Results> Copy for TypedFunc<Params, Results> {}

impl<Params, Results> Clone for TypedFunc<Params, Results> {
    fn clone(&self) -> TypedFunc<Params, Results> {
        *self
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
    pub unsafe fn new_unchecked(func: Func) -> TypedFunc<Params, Results> {
        TypedFunc {
            _a: marker::PhantomData,
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
    /// # Panics
    ///
    /// This function will panic if it is called when the underlying [`Func`] is
    /// connected to an asynchronous store.
    pub fn call(&self, mut store: impl AsContextMut, params: Params) -> Result<Results, Trap> {
        let mut store = store.as_context_mut();
        assert!(
            !store.0.async_support(),
            "must use `call_async` with async stores"
        );
        let func = self.func.caller_checked_anyfunc(store.0);
        unsafe { Self::call_raw(&mut store, func, params) }
    }

    /// Invokes this WebAssembly function with the specified parameters.
    ///
    /// Returns either the results of the call, or a [`Trap`] if one happened.
    ///
    /// For more information, see the [`Func::typed`] and [`Func::call_async`]
    /// documentation.
    ///
    /// # Panics
    ///
    /// This function will panic if it is called when the underlying [`Func`] is
    /// connected to a synchronous store.
    #[cfg(feature = "async")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    pub async fn call_async<T>(
        &self,
        mut store: impl AsContextMut<Data = T>,
        params: Params,
    ) -> Result<Results, Trap>
    where
        T: Send,
    {
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "must use `call` with non-async stores"
        );
        store
            .on_fiber(|store| {
                let func = self.func.caller_checked_anyfunc(store.0);
                unsafe { Self::call_raw(store, func, params) }
            })
            .await?
    }

    pub(crate) unsafe fn call_raw<T>(
        store: &mut StoreContextMut<'_, T>,
        func: ptr::NonNull<VMCallerCheckedAnyfunc>,
        params: Params,
    ) -> Result<Results, Trap> {
        // double-check that params/results match for this function's type in
        // debug mode.
        if cfg!(debug_assertions) {
            Self::debug_typecheck(store.0, func.as_ref().type_index);
        }

        // See the comment in `Func::call_impl`'s `write_params` function.
        if params.externrefs_count()
            > store
                .0
                .externref_activations_table()
                .bump_capacity_remaining()
        {
            store.gc();
        }

        // Validate that all runtime values flowing into this store indeed
        // belong within this store, otherwise it would be unsafe for store
        // values to cross each other.

        let params = {
            // GC is not safe here, since we move refs into the activations
            // table but don't hold a strong reference onto them until we enter
            // the Wasm frame and they get referenced from the stack maps.
            let mut store = AutoAssertNoGc::new(&mut **store.as_context_mut().0);

            match params.into_abi(&mut store) {
                Some(abi) => abi,
                None => {
                    return Err(Trap::new(
                        "attempt to pass cross-`Store` value to Wasm as function argument",
                    ))
                }
            }
        };

        // Try to capture only a single variable (a tuple) in the closure below.
        // This means the size of the closure is one pointer and is much more
        // efficient to move in memory. This closure is actually invoked on the
        // other side of a C++ shim, so it can never be inlined enough to make
        // the memory go away, so the size matters here for performance.
        let mut captures = (func, MaybeUninit::uninit(), params, false);

        let result = invoke_wasm_and_catch_traps(store, |callee| {
            let (anyfunc, ret, params, returned) = &mut captures;
            let anyfunc = anyfunc.as_ref();
            let result = Params::invoke::<Results>(
                anyfunc.func_ptr.as_ptr(),
                anyfunc.vmctx,
                callee,
                *params,
            );
            ptr::write(ret.as_mut_ptr(), result);
            *returned = true
        });
        let (_, ret, _, returned) = captures;
        debug_assert_eq!(result.is_ok(), returned);
        result?;
        Ok(Results::from_abi(store.0, ret.assume_init()))
    }

    /// Purely a debug-mode assertion, not actually used in release builds.
    fn debug_typecheck(store: &StoreOpaque, func: VMSharedSignatureIndex) {
        let ty = FuncType::from_wasm_func_type(
            store
                .engine()
                .signatures()
                .lookup_type(func)
                .expect("signature should be registered"),
        );
        Params::typecheck(ty.params()).expect("params should match");
        Results::typecheck(ty.results()).expect("results should match");
    }
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
    #[doc(hidden)]
    type Abi: Copy;
    #[doc(hidden)]
    #[inline]
    fn typecheck(ty: crate::ValType) -> Result<()> {
        if ty == Self::valtype() {
            Ok(())
        } else {
            bail!("expected {} found {}", Self::valtype(), ty)
        }
    }
    #[doc(hidden)]
    fn valtype() -> ValType;
    #[doc(hidden)]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool;
    #[doc(hidden)]
    fn is_externref(&self) -> bool;
    #[doc(hidden)]
    unsafe fn abi_from_raw(raw: *mut ValRaw) -> Self::Abi;
    #[doc(hidden)]
    unsafe fn abi_into_raw(abi: Self::Abi, raw: *mut ValRaw);
    #[doc(hidden)]
    fn into_abi(self, store: &mut StoreOpaque) -> Self::Abi;
    #[doc(hidden)]
    unsafe fn from_abi(abi: Self::Abi, store: &mut StoreOpaque) -> Self;
}

macro_rules! integers {
    ($($primitive:ident/$get_primitive:ident => $ty:ident in $raw:ident)*) => ($(
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
            fn is_externref(&self) -> bool {
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
            fn into_abi(self, _store: &mut StoreOpaque) -> Self::Abi {
                self
            }
            #[inline]
            unsafe fn from_abi(abi: Self::Abi, _store: &mut StoreOpaque) -> Self {
                abi
            }
        }
    )*)
}

integers! {
    i32/get_i32 => I32 in i32
    i64/get_i64 => I64 in i64
    u32/get_u32 => I32 in i32
    u64/get_u64 => I64 in i64
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
            fn is_externref(&self) -> bool {
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
            fn into_abi(self, _store: &mut StoreOpaque) -> Self::Abi {
                self
            }
            #[inline]
            unsafe fn from_abi(abi: Self::Abi, _store: &mut StoreOpaque) -> Self {
                abi
            }
        }
    )*)
}

floats! {
    f32/u32/get_f32 => F32
    f64/u64/get_f64 => F64
}

unsafe impl WasmTy for Option<ExternRef> {
    type Abi = *mut u8;

    #[inline]
    fn valtype() -> ValType {
        ValType::ExternRef
    }

    #[inline]
    fn compatible_with_store(&self, _store: &StoreOpaque) -> bool {
        true
    }

    #[inline]
    fn is_externref(&self) -> bool {
        true
    }

    #[inline]
    unsafe fn abi_from_raw(raw: *mut ValRaw) -> *mut u8 {
        (*raw).get_externref() as *mut u8
    }

    #[inline]
    unsafe fn abi_into_raw(abi: *mut u8, raw: *mut ValRaw) {
        *raw = ValRaw::externref(abi as usize);
    }

    #[inline]
    fn into_abi(self, store: &mut StoreOpaque) -> Self::Abi {
        if let Some(x) = self {
            let abi = x.inner.as_raw();
            unsafe {
                // NB: We _must not_ trigger a GC when passing refs from host
                // code into Wasm (e.g. returned from a host function or passed
                // as arguments to a Wasm function). After insertion into the
                // table, this reference is no longer rooted. If multiple
                // references are being sent from the host into Wasm and we
                // allowed GCs during insertion, then the following events could
                // happen:
                //
                // * Reference A is inserted into the activations
                //   table. This does not trigger a GC, but does fill the table
                //   to capacity.
                //
                // * The caller's reference to A is removed. Now the only
                //   reference to A is from the activations table.
                //
                // * Reference B is inserted into the activations table. Because
                //   the table is at capacity, a GC is triggered.
                //
                // * A is reclaimed because the only reference keeping it alive
                //   was the activation table's reference (it isn't inside any
                //   Wasm frames on the stack yet, so stack scanning and stack
                //   maps don't increment its reference count).
                //
                // * We transfer control to Wasm, giving it A and B. Wasm uses
                //   A. That's a use after free.
                //
                // In conclusion, to prevent uses after free, we cannot GC
                // during this insertion.
                let mut store = AutoAssertNoGc::new(store);
                store.insert_vmexternref_without_gc(x.inner);
            }
            abi
        } else {
            ptr::null_mut()
        }
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, _store: &mut StoreOpaque) -> Self {
        if abi.is_null() {
            None
        } else {
            Some(ExternRef {
                inner: wasmtime_runtime::VMExternRef::clone_from_raw(abi),
            })
        }
    }
}

unsafe impl WasmTy for Option<Func> {
    type Abi = *mut wasmtime_runtime::VMCallerCheckedAnyfunc;

    #[inline]
    fn valtype() -> ValType {
        ValType::FuncRef
    }

    #[inline]
    fn compatible_with_store<'a>(&self, store: &StoreOpaque) -> bool {
        if let Some(f) = self {
            store.store_data().contains(f.0)
        } else {
            true
        }
    }

    #[inline]
    fn is_externref(&self) -> bool {
        false
    }

    #[inline]
    unsafe fn abi_from_raw(raw: *mut ValRaw) -> Self::Abi {
        (*raw).get_funcref() as Self::Abi
    }

    #[inline]
    unsafe fn abi_into_raw(abi: Self::Abi, raw: *mut ValRaw) {
        *raw = ValRaw::funcref(abi as usize);
    }

    #[inline]
    fn into_abi(self, store: &mut StoreOpaque) -> Self::Abi {
        if let Some(f) = self {
            f.caller_checked_anyfunc(store).as_ptr()
        } else {
            ptr::null_mut()
        }
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, store: &mut StoreOpaque) -> Self {
        Func::from_caller_checked_anyfunc(store, abi)
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
    fn typecheck(params: impl ExactSizeIterator<Item = crate::ValType>) -> Result<()>;

    #[doc(hidden)]
    fn externrefs_count(&self) -> usize;

    #[doc(hidden)]
    fn into_abi(self, store: &mut StoreOpaque) -> Option<Self::Abi>;

    #[doc(hidden)]
    unsafe fn invoke<R: WasmResults>(
        func: *const VMFunctionBody,
        vmctx1: *mut VMContext,
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

    fn typecheck(params: impl ExactSizeIterator<Item = crate::ValType>) -> Result<()> {
        <(T,) as WasmParams>::typecheck(params)
    }

    #[inline]
    fn externrefs_count(&self) -> usize {
        T::is_externref(self) as usize
    }

    #[inline]
    fn into_abi(self, store: &mut StoreOpaque) -> Option<Self::Abi> {
        <(T,) as WasmParams>::into_abi((self,), store)
    }

    unsafe fn invoke<R: WasmResults>(
        func: *const VMFunctionBody,
        vmctx1: *mut VMContext,
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

            fn typecheck(mut params: impl ExactSizeIterator<Item = crate::ValType>) -> Result<()> {
                let mut _n = 0;

                $(
                    match params.next() {
                        Some(t) => {
                            _n += 1;
                            $t::typecheck(t)?
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
            fn externrefs_count(&self) -> usize {
                let ($(ref $t,)*) = self;
                0 $(
                    + $t.is_externref() as usize
                )*
            }


            #[inline]
            fn into_abi(self, _store: &mut StoreOpaque) -> Option<Self::Abi> {
                let ($($t,)*) = self;
                $(
                    let $t = if $t.compatible_with_store(_store) {
                        $t.into_abi(_store)
                    } else {
                        return None;
                    };
                )*
                Some(($($t,)*))
            }

            unsafe fn invoke<R: WasmResults>(
                func: *const VMFunctionBody,
                vmctx1: *mut VMContext,
                vmctx2: *mut VMContext,
                abi: Self::Abi,
            ) -> R::ResultAbi {
                let fnptr = mem::transmute::<
                    *const VMFunctionBody,
                    unsafe extern "C" fn(
                        *mut VMContext,
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
///
/// This is currently only implemented for `()` and for bare types that can be
/// returned. This is not yet implemented for tuples because a multi-value
/// `TypedFunc` is not currently supported.
pub unsafe trait WasmResults: WasmParams {
    #[doc(hidden)]
    type ResultAbi: HostAbi;
    #[doc(hidden)]
    unsafe fn from_abi(store: &mut StoreOpaque, abi: Self::ResultAbi) -> Self;
}

// Forwards from a bare type `T` to the 1-tuple type `(T,)`
unsafe impl<T: WasmTy> WasmResults for T
where
    (T::Abi,): HostAbi,
{
    type ResultAbi = <(T,) as WasmResults>::ResultAbi;

    unsafe fn from_abi(store: &mut StoreOpaque, abi: Self::ResultAbi) -> Self {
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
            unsafe fn from_abi(store: &mut StoreOpaque, abi: Self::ResultAbi) -> Self {
                let ($($t,)*) = abi;
                ($($t::from_abi($t, store),)*)
            }
        }
    };
}

for_each_function_signature!(impl_wasm_results);
