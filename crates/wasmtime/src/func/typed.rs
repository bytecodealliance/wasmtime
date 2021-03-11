use super::invoke_wasm_and_catch_traps;
use crate::{ExternRef, Func, Store, Trap, ValType};
use anyhow::{bail, Result};
use std::marker;
use std::mem::{self, MaybeUninit};
use std::ptr;
use wasmtime_runtime::{VMContext, VMFunctionBody, VMTrampoline};

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
            !self.func.store().async_support(),
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

        let anyfunc = self.func.export.anyfunc.as_ref();
        let trampoline = self.func.trampoline;
        let params = MaybeUninit::new(params);
        let mut ret = MaybeUninit::uninit();
        let mut called = false;
        let mut returned = false;
        let result = invoke_wasm_and_catch_traps(&self.func.instance.store, || {
            called = true;
            let params = ptr::read(params.as_ptr());
            let result = params.invoke::<Results>(
                &self.func.instance.store,
                trampoline,
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
                    .insert_with_gc(x.inner, store.stack_map_registry());
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
        trampoline: VMTrampoline,
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
        trampoline: VMTrampoline,
        func: *const VMFunctionBody,
        vmctx1: *mut VMContext,
        vmctx2: *mut VMContext,
    ) -> R {
        <(T,)>::invoke((self,), store, trampoline, func, vmctx1, vmctx2)
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
                trampoline: VMTrampoline,
                func: *const VMFunctionBody,
                vmctx1: *mut VMContext,
                vmctx2: *mut VMContext,
            ) -> R {
                // Some signatures can go directly into JIT code which uses the
                // default platform ABI, but basically only those without
                // multiple return values. With multiple return values we can't
                // natively in Rust call such a function because there's no way
                // to model it (yet).
                //
                // To work around that we use the trampoline which passes
                // arguments/values via the stack which allows us to match the
                // expected ABI. Note that this branch, using the trampoline,
                // is slower as a result and has an extra indirect function
                // call as well. In the future if this is a problem we should
                // consider updating JIT code to use an ABI we can call from
                // Rust itself.
                if R::uses_trampoline() {
                    R::with_space(|space1| {
                        // Figure out whether the parameters or the results
                        // require more space, and use the bigger one as where
                        // to store arguments and load return values from.
                        let mut space2 = [0; $n];
                        let space = if space1.len() < space2.len() {
                            space2.as_mut_ptr()
                        } else {
                            space1.as_mut_ptr()
                        };

                        // ... store the ABI for all values into our storage
                        // area...
                        let ($($t,)*) = self;
                        let mut _n = 0;
                        $(
                            *space.add(_n).cast::<$t::Abi>() = $t.into_abi(store);
                            _n += 1;
                        )*

                        // ... make the indirect call through the trampoline
                        // which will read from `space` and also write all the
                        // results to `space`...
                        trampoline(vmctx1, vmctx2, func, space);

                        // ... and then we can decode all the return values
                        // from `space`.
                        R::from_storage(space, store)
                    })
                } else {
                    let fnptr = mem::transmute::<
                        *const VMFunctionBody,
                        unsafe extern "C" fn(
                            *mut VMContext,
                            *mut VMContext,
                            $($t::Abi,)*
                        ) -> R::Abi,
                    >(func);
                    let ($($t,)*) = self;
                    R::from_abi(fnptr(vmctx1, vmctx2, $($t.into_abi(store),)*), store)
                }
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
    type Abi;
    #[doc(hidden)]
    unsafe fn from_abi(abi: Self::Abi, store: &Store) -> Self;
    #[doc(hidden)]
    fn uses_trampoline() -> bool;
    // Provides a stack-allocated array with enough space to store all these
    // result values.
    //
    // It'd be nice if we didn't have to have this API and could do something
    // with const-generics (or something like that), but I couldn't figure it
    // out. If a future Rust explorer is able to get something like `const LEN:
    // usize` working that'd be great!
    #[doc(hidden)]
    fn with_space<R>(_: impl FnOnce(&mut [u128]) -> R) -> R;
    #[doc(hidden)]
    unsafe fn from_storage(ptr: *const u128, store: &Store) -> Self;
}

unsafe impl<T: WasmTy> WasmResults for T {
    type Abi = <(T,) as WasmResults>::Abi;
    unsafe fn from_abi(abi: Self::Abi, store: &Store) -> Self {
        <(T,) as WasmResults>::from_abi(abi, store).0
    }
    fn uses_trampoline() -> bool {
        <(T,) as WasmResults>::uses_trampoline()
    }
    fn with_space<R>(f: impl FnOnce(&mut [u128]) -> R) -> R {
        <(T,) as WasmResults>::with_space(f)
    }
    unsafe fn from_storage(ptr: *const u128, store: &Store) -> Self {
        <(T,) as WasmResults>::from_storage(ptr, store).0
    }
}

#[doc(hidden)]
pub enum Void {}

macro_rules! impl_wasm_results {
    ($n:tt $($t:ident)*) => {
        #[allow(non_snake_case, unused_variables)]
        unsafe impl<$($t: WasmTy,)*> WasmResults for ($($t,)*) {
            type Abi = impl_wasm_results!(@abi $n $($t)*);
            unsafe fn from_abi(abi: Self::Abi, store: &Store) -> Self {
                impl_wasm_results!(@from_abi abi store $n $($t)*)
            }
            fn uses_trampoline() -> bool {
                $n > 1
            }
            fn with_space<R>(f: impl FnOnce(&mut [u128]) -> R) -> R {
                f(&mut [0; $n])
            }
            unsafe fn from_storage(ptr: *const u128, store: &Store) -> Self {
                let mut _n = 0;
                $(
                    let $t = $t::from_abi(*ptr.add(_n).cast::<$t::Abi>(), store);
                    _n += 1;
                )*
                ($($t,)*)
            }
        }
    };

    // 0/1 return values we can use natively, everything else isn't expressible
    // and won't be used so define the abi type to Void.
    (@abi 0) => (());
    (@abi 1 $t:ident) => ($t::Abi);
    (@abi $($t:tt)*) => (Void);

    (@from_abi $abi:ident $store:ident 0) => (());
    (@from_abi $abi:ident $store:ident 1 $t:ident) => (($t::from_abi($abi, $store),));
    (@from_abi $abi:ident $store:ident $($t:tt)*) => ({
        debug_assert!(false);
        match $abi {}
    });
}

for_each_function_signature!(impl_wasm_results);
