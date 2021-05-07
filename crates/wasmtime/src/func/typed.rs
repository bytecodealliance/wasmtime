use super::{invoke_wasm_and_catch_traps, HostAbi};
use crate::{ExternRef, Func, Store, Trap, ValType};
use anyhow::{bail, Result};
use std::marker;
use std::mem::{self, MaybeUninit};
use std::ptr;
use wasmtime_runtime::{VMContext, VMFunctionBody};

/// A statically typed WebAssembly function.
///
/// Values of this type represent statically type-checked WebAssembly functions.
/// The function within a [`TypedFunc`] is statically known to have `Params` as its
/// parameters and `Results` as its results.
///
/// This structure is created via [`Func::typed`] or [`Func::typed_unchecked`].
/// For more documentation about this see those methods.
#[repr(transparent)]
pub struct TypedFunc<Params, Results> {
    _a: marker::PhantomData<fn(Params) -> Results>,
    func: Func,
}

impl<Params, Results> Clone for TypedFunc<Params, Results> {
    fn clone(&self) -> TypedFunc<Params, Results> {
        TypedFunc {
            _a: marker::PhantomData,
            func: self.func.clone(),
        }
    }
}

impl<Params, Results> TypedFunc<Params, Results>
where
    Params: WasmParams,
    Results: WasmResults,
{
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
    pub fn call(&self, params: Params) -> Result<Results, Trap> {
        assert!(
            !cfg!(feature = "async") || !self.func.store().async_support(),
            "must use `call_async` with async stores"
        );
        unsafe { self._call(params) }
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
    pub async fn call_async(&self, params: Params) -> Result<Results, Trap> {
        assert!(
            self.func.store().async_support(),
            "must use `call` with non-async stores"
        );
        self.func
            .store()
            .on_fiber(|| unsafe { self._call(params) })
            .await?
    }

    unsafe fn _call(&self, params: Params) -> Result<Results, Trap> {
        // Validate that all runtime values flowing into this store indeed
        // belong within this store, otherwise it would be unsafe for store
        // values to cross each other.
        if !params.compatible_with_store(&self.func.instance.store) {
            return Err(Trap::new(
                "attempt to pass cross-`Store` value to Wasm as function argument",
            ));
        }

        let params = MaybeUninit::new(params);
        let mut ret = MaybeUninit::uninit();
        let mut called = false;
        let mut returned = false;
        let result = invoke_wasm_and_catch_traps(&self.func.instance.store, || {
            called = true;
            let params = ptr::read(params.as_ptr());
            let anyfunc = self.func.export.anyfunc.as_ref();
            let result = params.invoke::<Results>(
                &self.func.instance.store,
                anyfunc.func_ptr.as_ptr(),
                anyfunc.vmctx,
                ptr::null_mut(),
            );
            ptr::write(ret.as_mut_ptr(), result);
            returned = true
        });

        // This can happen if we early-trap due to interrupts or other
        // pre-flight checks, so we need to be sure the parameters are at least
        // dropped at some point.
        if !called {
            drop(params.assume_init());
        }
        debug_assert_eq!(result.is_ok(), returned);
        result?;

        Ok(ret.assume_init())
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
pub unsafe trait WasmTy {
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
    fn compatible_with_store(&self, store: &Store) -> bool;
    #[doc(hidden)]
    fn into_abi(self, store: &Store) -> Self::Abi;
    #[doc(hidden)]
    unsafe fn from_abi(abi: Self::Abi, store: &Store) -> Self;
}

macro_rules! primitives {
    ($($primitive:ident => $ty:ident)*) => ($(
        unsafe impl WasmTy for $primitive {
            type Abi = $primitive;
            #[inline]
            fn valtype() -> ValType {
                ValType::$ty
            }
            #[inline]
            fn compatible_with_store(&self, _: &Store) -> bool {
                true
            }
            #[inline]
            fn into_abi(self, _store: &Store) -> Self::Abi {
                self
            }
            #[inline]
            unsafe fn from_abi(abi: Self::Abi, _store: &Store) -> Self {
                abi
            }
        }
    )*)
}

primitives! {
    i32 => I32
    u32 => I32
    i64 => I64
    u64 => I64
    f32 => F32
    f64 => F64
}

unsafe impl WasmTy for Option<ExternRef> {
    type Abi = *mut u8;

    #[inline]
    fn valtype() -> ValType {
        ValType::ExternRef
    }

    #[inline]
    fn compatible_with_store(&self, _store: &Store) -> bool {
        true
    }

    #[inline]
    fn into_abi(self, store: &Store) -> Self::Abi {
        if let Some(x) = self {
            let abi = x.inner.as_raw();
            unsafe {
                store
                    .externref_activations_table()
                    .insert_with_gc(x.inner, store.module_info_lookup());
            }
            abi
        } else {
            ptr::null_mut()
        }
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, _store: &Store) -> Self {
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
    fn compatible_with_store<'a>(&self, store: &Store) -> bool {
        if let Some(f) = self {
            Store::same(&store, f.store())
        } else {
            true
        }
    }

    #[inline]
    fn into_abi(self, _store: &Store) -> Self::Abi {
        if let Some(f) = self {
            f.caller_checked_anyfunc().as_ptr()
        } else {
            ptr::null_mut()
        }
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, store: &Store) -> Self {
        Func::from_caller_checked_anyfunc(&store, abi)
    }
}

/// A trait used for [`Func::typed`] and with [`TypedFunc`] to represent the set of
/// parameters for wasm functions.
///
/// This is implemented for bare types that can be passed to wasm as well as
/// tuples of those types.
pub unsafe trait WasmParams {
    #[doc(hidden)]
    fn typecheck(params: impl ExactSizeIterator<Item = crate::ValType>) -> Result<()>;
    #[doc(hidden)]
    fn compatible_with_store(&self, store: &Store) -> bool;
    #[doc(hidden)]
    unsafe fn invoke<R: WasmResults>(
        self,
        store: &Store,
        func: *const VMFunctionBody,
        vmctx1: *mut VMContext,
        vmctx2: *mut VMContext,
    ) -> R;
}

// Forward an impl from `T` to `(T,)` for convenience if there's only one
// parameter.
unsafe impl<T> WasmParams for T
where
    T: WasmTy,
{
    fn typecheck(params: impl ExactSizeIterator<Item = crate::ValType>) -> Result<()> {
        <(T,)>::typecheck(params)
    }
    fn compatible_with_store(&self, store: &Store) -> bool {
        <T as WasmTy>::compatible_with_store(self, store)
    }
    unsafe fn invoke<R: WasmResults>(
        self,
        store: &Store,
        func: *const VMFunctionBody,
        vmctx1: *mut VMContext,
        vmctx2: *mut VMContext,
    ) -> R {
        <(T,)>::invoke((self,), store, func, vmctx1, vmctx2)
    }
}

macro_rules! impl_wasm_params {
    ($n:tt $($t:ident)*) => {
        #[allow(non_snake_case)]
        unsafe impl<$($t: WasmTy,)*> WasmParams for ($($t,)*) {
            fn typecheck(mut params: impl ExactSizeIterator<Item = crate::ValType>) -> Result<()> {
                let mut _n = 0;
                $(
                    match params.next() {
                        Some(t) => $t::typecheck(t)?,
                        None => bail!("expected {} types, found {}", $n, _n),
                    }
                    _n += 1;
                )*

                match params.next() {
                    None => Ok(()),
                    Some(_) => bail!("expected {} types, found {}", $n, params.len() + _n),
                }
            }

            fn compatible_with_store(&self, _store: &Store) -> bool {
                let ($($t,)*) = self;
                $($t.compatible_with_store(_store)&&)* true
            }

            unsafe fn invoke<R: WasmResults>(
                self,
                store: &Store,
                func: *const VMFunctionBody,
                vmctx1: *mut VMContext,
                vmctx2: *mut VMContext,
            ) -> R {
                let fnptr = mem::transmute::<
                    *const VMFunctionBody,
                    unsafe extern "C" fn(
                        *mut VMContext,
                        *mut VMContext,
                        $($t::Abi,)*
                        R::Retptr,
                    ) -> R::Abi,
                >(func);
                let ($($t,)*) = self;
                // Use the `call` function to acquire a `retptr` which we'll
                // forward to the native function. Once we have it we also
                // convert all our arguments to abi arguments to go to the raw
                // function.
                //
                // Upon returning `R::call` will convert all the returns back
                // into `R`.
                R::call(store, |retptr| {
                    fnptr(vmctx1, vmctx2, $($t.into_abi(store),)* retptr)
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
    type Abi: Copy;
    #[doc(hidden)]
    type Retptr: Copy;
    #[doc(hidden)]
    unsafe fn call(store: &Store, f: impl FnOnce(Self::Retptr) -> Self::Abi) -> Self;
}

// Forwards from a bare type `T` to the 1-tuple type `(T,)`
unsafe impl<T: WasmTy> WasmResults for T
where
    (T::Abi,): HostAbi,
{
    type Abi = <(T,) as WasmResults>::Abi;
    type Retptr = <(T,) as WasmResults>::Retptr;

    unsafe fn call(store: &Store, f: impl FnOnce(Self::Retptr) -> Self::Abi) -> Self {
        <(T,) as WasmResults>::call(store, f).0
    }
}

macro_rules! impl_wasm_results {
    ($n:tt $($t:ident)*) => {
        #[allow(non_snake_case, unused_variables)]
        unsafe impl<$($t: WasmTy,)*> WasmResults for ($($t,)*)
            where ($($t::Abi,)*): HostAbi
        {
            type Abi = <($($t::Abi,)*) as HostAbi>::Abi;
            type Retptr = <($($t::Abi,)*) as HostAbi>::Retptr;

            unsafe fn call(store: &Store, f: impl FnOnce(Self::Retptr) -> Self::Abi) -> Self {
                // Delegate via the host abi to figure out what the actual ABI
                // for dealing with this tuple type is, and then we can re-tuple
                // everything and create actual values via `from_abi` after the
                // call is complete.
                let ($($t,)*) = <($($t::Abi,)*) as HostAbi>::call(f);
                ($($t::from_abi($t, store),)*)
            }
        }
    };
}

for_each_function_signature!(impl_wasm_results);
