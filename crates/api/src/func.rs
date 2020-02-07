use crate::callable::{NativeCallable, WasmtimeFn, WrappedCallable};
use crate::{Callable, FuncType, Store, Trap, Val, ValType};
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;
use wasmtime_jit::InstanceHandle;
use wasmtime_runtime::VMContext;

/// A WebAssembly function which can be called.
///
/// This type can represent a number of callable items, such as:
///
/// * An exported function from a WebAssembly module.
/// * A user-defined function used to satisfy an import.
///
/// These types of callable items are all wrapped up in this `Func` and can be
/// used to both instantiate an [`Instance`](crate::Instance) as well as be
/// extracted from an [`Instance`](crate::Instance).
///
/// # `Func` and `Clone`
///
/// Functions are internally reference counted so you can `clone` a `Func`. The
/// cloning process only performs a shallow clone, so two cloned `Func`
/// instances are equivalent in their functionality.
#[derive(Clone)]
pub struct Func {
    _store: Store,
    callable: Rc<dyn WrappedCallable + 'static>,
    ty: FuncType,
}

macro_rules! wrappers {
    ($(
        $(#[$doc:meta])*
        ($name:ident $(,$args:ident)*)
    )*) => ($(
        $(#[$doc])*
        pub fn $name<F, $($args,)* R>(store: &Store, func: F) -> Func
        where
            F: Fn($($args),*) -> R + 'static,
            $($args: WasmArg,)*
            R: WasmRet,
        {
            #[allow(non_snake_case)]
            unsafe extern "C" fn shim<F, $($args,)* R>(
                vmctx: *mut VMContext,
                _caller_vmctx: *mut VMContext,
                $($args: $args::Abi,)*
            ) -> R::Abi
            where
                F: Fn($($args),*) -> R + 'static,
                $($args: WasmArg,)*
                R: WasmRet,
            {
                let ret = {
                    let instance = InstanceHandle::from_vmctx(vmctx);
                    let func = instance.host_state().downcast_ref::<F>().expect("state");
                    panic::catch_unwind(AssertUnwindSafe(|| {
                        func($($args::from_abi(_caller_vmctx, $args)),*)
                    }))
                };
                match ret {
                    Ok(ret) => ret.into_abi(),
                    Err(panic) => wasmtime_runtime::resume_panic(panic),
                }
            }

            let mut _args = Vec::new();
            $($args::push(&mut _args);)*
            let mut ret = Vec::new();
            R::push(&mut ret);
            let ty = FuncType::new(_args.into(), ret.into());
            unsafe {
                let (instance, export) = crate::trampoline::generate_raw_func_export(
                    &ty,
                    std::slice::from_raw_parts_mut(
                        shim::<F, $($args,)* R> as *mut _,
                        0,
                    ),
                    store,
                    Box::new(func),
                )
                .expect("failed to generate export");
                let callable = Rc::new(WasmtimeFn::new(store, instance, export));
                Func::from_wrapped(store, ty, callable)
            }
        }
    )*)
}

impl Func {
    /// Creates a new `Func` with the given arguments, typically to create a
    /// user-defined function to pass as an import to a module.
    ///
    /// * `store` - a cache of data where information is stored, typically
    ///   shared with a [`Module`](crate::Module).
    ///
    /// * `ty` - the signature of this function, used to indicate what the
    ///   inputs and outputs are, which must be WebAssembly types.
    ///
    /// * `callable` - a type implementing the [`Callable`] trait which
    ///   is the implementation of this `Func` value.
    ///
    /// Note that the implementation of `callable` must adhere to the `ty`
    /// signature given, error or traps may occur if it does not respect the
    /// `ty` signature.
    pub fn new(store: &Store, ty: FuncType, callable: Rc<dyn Callable + 'static>) -> Self {
        let callable = Rc::new(NativeCallable::new(callable, &ty, &store));
        Func::from_wrapped(store, ty, callable)
    }

    wrappers! {
        /// Creates a new `Func` from the given Rust closure, which takes 0
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap0)

        /// Creates a new `Func` from the given Rust closure, which takes 1
        /// argument.
        ///
        /// This function will create a new `Func` which, when called, will
        /// execute the given Rust closure. Unlike [`Func::new`] the target
        /// function being called is known statically so the type signature can
        /// be inferred. Rust types will map to WebAssembly types as follows:
        ///
        ///
        /// | Rust Argument Type | WebAssembly Type |
        /// |--------------------|------------------|
        /// | `i32`              | `i32`            |
        /// | `i64`              | `i64`            |
        /// | `f32`              | `f32`            |
        /// | `f64`              | `f64`            |
        /// | (not supported)    | `v128`           |
        /// | (not supported)    | `anyref`         |
        ///
        /// Any of the Rust types can be returned from the closure as well, in
        /// addition to some extra types
        ///
        /// | Rust Return Type  | WebAssembly Return Type | Meaning           |
        /// |-------------------|-------------------------|-------------------|
        /// | `()`              | nothing                 | no return value   |
        /// | `Result<T, Trap>` | `T`                     | function may trap |
        ///
        /// Note that when using this API (and the related `wrap*` family of
        /// functions), the intention is to create as thin of a layer as
        /// possible for when WebAssembly calls the function provided. With
        /// sufficient inlining and optimization the WebAssembly will call
        /// straight into `func` provided, with no extra fluff entailed.
        (wrap1, A)

        /// Creates a new `Func` from the given Rust closure, which takes 2
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap2, A, B)

        /// Creates a new `Func` from the given Rust closure, which takes 3
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap3, A, B, C)

        /// Creates a new `Func` from the given Rust closure, which takes 4
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap4, A, B, C, D)

        /// Creates a new `Func` from the given Rust closure, which takes 5
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap5, A, B, C, D, E)

        /// Creates a new `Func` from the given Rust closure, which takes 6
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap6, A, B, C, D, E, G)

        /// Creates a new `Func` from the given Rust closure, which takes 7
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap7, A, B, C, D, E, G, H)

        /// Creates a new `Func` from the given Rust closure, which takes 8
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap8, A, B, C, D, E, G, H, I)

        /// Creates a new `Func` from the given Rust closure, which takes 9
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap9, A, B, C, D, E, G, H, I, J)

        /// Creates a new `Func` from the given Rust closure, which takes 10
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap10, A, B, C, D, E, G, H, I, J, K)
    }

    fn from_wrapped(
        store: &Store,
        ty: FuncType,
        callable: Rc<dyn WrappedCallable + 'static>,
    ) -> Func {
        Func {
            _store: store.clone(),
            callable,
            ty,
        }
    }

    /// Returns the underlying wasm type that this `Func` has.
    pub fn ty(&self) -> &FuncType {
        &self.ty
    }

    /// Returns the number of parameters that this function takes.
    pub fn param_arity(&self) -> usize {
        self.ty.params().len()
    }

    /// Returns the number of results this function produces.
    pub fn result_arity(&self) -> usize {
        self.ty.results().len()
    }

    /// Invokes this function with the `params` given, returning the results and
    /// any trap, if one occurs.
    ///
    /// The `params` here must match the type signature of this `Func`, or a
    /// trap will occur. If a trap occurs while executing this function, then a
    /// trap will also be returned.
    ///
    /// This function should not panic unless the underlying function itself
    /// initiates a panic.
    pub fn call(&self, params: &[Val]) -> Result<Box<[Val]>, Trap> {
        let mut results = vec![Val::null(); self.result_arity()];
        self.callable.call(params, &mut results)?;
        Ok(results.into_boxed_slice())
    }

    pub(crate) fn wasmtime_export(&self) -> &wasmtime_runtime::Export {
        self.callable.wasmtime_export()
    }

    pub(crate) fn from_wasmtime_function(
        export: wasmtime_runtime::Export,
        store: &Store,
        instance_handle: InstanceHandle,
    ) -> Self {
        // This is only called with `Export::Function`, and since it's coming
        // from wasmtime_runtime itself we should support all the types coming
        // out of it, so assert such here.
        let ty = if let wasmtime_runtime::Export::Function { signature, .. } = &export {
            FuncType::from_wasmtime_signature(signature.clone())
                .expect("core wasm signature should be supported")
        } else {
            panic!("expected function export")
        };
        let callable = WasmtimeFn::new(store, instance_handle, export);
        Func::from_wrapped(store, ty, Rc::new(callable))
    }
}

impl fmt::Debug for Func {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Func")
    }
}

/// A trait implemented for types which can be arguments to closures passed to
/// [`Func::wrap1`] and friends.
///
/// This trait should not be implemented by user types. This trait may change at
/// any time internally. The types which implement this trait, however, are
/// stable over time.
///
/// For more information see [`Func::wrap1`]
pub trait WasmArg {
    #[doc(hidden)]
    type Abi;
    #[doc(hidden)]
    fn push(dst: &mut Vec<ValType>);
    #[doc(hidden)]
    fn from_abi(vmctx: *mut VMContext, abi: Self::Abi) -> Self;
}

impl WasmArg for () {
    type Abi = ();
    fn push(_dst: &mut Vec<ValType>) {}
    #[inline]
    fn from_abi(_vmctx: *mut VMContext, abi: Self::Abi) -> Self {
        abi
    }
}

impl WasmArg for i32 {
    type Abi = Self;
    fn push(dst: &mut Vec<ValType>) {
        dst.push(ValType::I32);
    }
    #[inline]
    fn from_abi(_vmctx: *mut VMContext, abi: Self::Abi) -> Self {
        abi
    }
}

impl WasmArg for i64 {
    type Abi = Self;
    fn push(dst: &mut Vec<ValType>) {
        dst.push(ValType::I64);
    }
    #[inline]
    fn from_abi(_vmctx: *mut VMContext, abi: Self::Abi) -> Self {
        abi
    }
}

impl WasmArg for f32 {
    type Abi = Self;
    fn push(dst: &mut Vec<ValType>) {
        dst.push(ValType::F32);
    }
    #[inline]
    fn from_abi(_vmctx: *mut VMContext, abi: Self::Abi) -> Self {
        abi
    }
}

impl WasmArg for f64 {
    type Abi = Self;
    fn push(dst: &mut Vec<ValType>) {
        dst.push(ValType::F64);
    }
    #[inline]
    fn from_abi(_vmctx: *mut VMContext, abi: Self::Abi) -> Self {
        abi
    }
}

/// A trait implemented for types which can be returned from closures passed to
/// [`Func::wrap1`] and friends.
///
/// This trait should not be implemented by user types. This trait may change at
/// any time internally. The types which implement this trait, however, are
/// stable over time.
///
/// For more information see [`Func::wrap1`]
pub trait WasmRet {
    #[doc(hidden)]
    type Abi;
    #[doc(hidden)]
    fn push(dst: &mut Vec<ValType>);
    #[doc(hidden)]
    fn into_abi(self) -> Self::Abi;
}

impl<T: WasmArg> WasmRet for T {
    type Abi = T;
    fn push(dst: &mut Vec<ValType>) {
        T::push(dst)
    }

    #[inline]
    fn into_abi(self) -> Self::Abi {
        self
    }
}

impl<T: WasmArg> WasmRet for Result<T, Trap> {
    type Abi = T;
    fn push(dst: &mut Vec<ValType>) {
        T::push(dst)
    }

    #[inline]
    fn into_abi(self) -> Self::Abi {
        match self {
            Ok(val) => return val,
            Err(trap) => handle_trap(trap),
        }

        fn handle_trap(trap: Trap) -> ! {
            unsafe { wasmtime_runtime::raise_user_trap(Box::new(trap)) }
        }
    }
}
