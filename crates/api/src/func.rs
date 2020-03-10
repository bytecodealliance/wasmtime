use crate::callable::{NativeCallable, WasmtimeFn, WrappedCallable};
use crate::{Callable, FuncType, Store, Trap, Val, ValType};
use anyhow::{ensure, Context as _};
use std::fmt;
use std::mem;
use std::panic::{self, AssertUnwindSafe};
use std::ptr;
use std::rc::Rc;
use wasmtime_runtime::{InstanceHandle, VMContext, VMFunctionBody};

/// A WebAssembly function which can be called.
///
/// This type can represent a number of callable items, such as:
///
/// * An exported function from a WebAssembly module.
/// * A user-defined function used to satisfy an import.
///
/// These types of callable items are all wrapped up in this `Func` and can be
/// used to both instantiate an [`Instance`] as well as be extracted from an
/// [`Instance`].
///
/// [`Instance`]: crate::Instance
///
/// # `Func` and `Clone`
///
/// Functions are internally reference counted so you can `clone` a `Func`. The
/// cloning process only performs a shallow clone, so two cloned `Func`
/// instances are equivalent in their functionality.
///
/// # Examples
///
/// One way to get a `Func` is from an [`Instance`] after you've instantiated
/// it:
///
/// ```
/// # use wasmtime::*;
/// # fn main() -> anyhow::Result<()> {
/// let store = Store::default();
/// let module = Module::new(&store, r#"(module (func (export "foo")))"#)?;
/// let instance = Instance::new(&module, &[])?;
/// let foo = instance.exports()[0].func().expect("export wasn't a function");
///
/// // Work with `foo` as a `Func` at this point, such as calling it
/// // dynamically...
/// match foo.call(&[]) {
///     Ok(result) => { /* ... */ }
///     Err(trap) => {
///         panic!("execution of `foo` resulted in a wasm trap: {}", trap);
///     }
/// }
/// foo.call(&[])?;
///
/// // ... or we can make a static assertion about its signature and call it.
/// // Our first call here can fail if the signatures don't match, and then the
/// // second call can fail if the function traps (like the `match` above).
/// let foo = foo.get0::<()>()?;
/// foo()?;
/// # Ok(())
/// # }
/// ```
///
/// You can also use the [`wrap*` family of functions](Func::wrap1) to create a
/// `Func`
///
/// ```
/// # use wasmtime::*;
/// # fn main() -> anyhow::Result<()> {
/// let store = Store::default();
///
/// // Create a custom `Func` which can execute arbitrary code inside of the
/// // closure.
/// let add = Func::wrap2(&store, |a: i32, b: i32| -> i32 { a + b });
///
/// // Next we can hook that up to a wasm module which uses it.
/// let module = Module::new(
///     &store,
///     r#"
///         (module
///             (import "" "" (func $add (param i32 i32) (result i32)))
///             (func (export "call_add_twice") (result i32)
///                 i32.const 1
///                 i32.const 2
///                 call $add
///                 i32.const 3
///                 i32.const 4
///                 call $add
///                 i32.add))
///     "#,
/// )?;
/// let instance = Instance::new(&module, &[add.into()])?;
/// let call_add_twice = instance.exports()[0].func().expect("export wasn't a function");
/// let call_add_twice = call_add_twice.get0::<i32>()?;
///
/// assert_eq!(call_add_twice()?, 10);
/// # Ok(())
/// # }
/// ```
///
/// Or you could also create an entirely dynamic `Func`!
///
/// ```
/// # use wasmtime::*;
/// use std::rc::Rc;
///
/// struct Double;
///
/// impl Callable for Double {
///     fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap> {
///         let mut value = params[0].unwrap_i32();
///         value *= 2;
///         results[0] = value.into();
///         Ok(())
///     }
/// }
///
/// # fn main() -> anyhow::Result<()> {
/// let store = Store::default();
///
/// // Here we need to define the type signature of our `Double` function and
/// // then wrap it up in a `Func`
/// let double_type = wasmtime::FuncType::new(
///     Box::new([wasmtime::ValType::I32]),
///     Box::new([wasmtime::ValType::I32])
/// );
/// let double = Func::new(&store, double_type, Rc::new(Double));
///
/// let module = Module::new(
///     &store,
///     r#"
///         (module
///             (import "" "" (func $double (param i32) (result i32)))
///             (func $start
///                 i32.const 1
///                 call $double
///                 drop)
///             (start $start))
///     "#,
/// )?;
/// let instance = Instance::new(&module, &[double.into()])?;
/// // .. work with `instance` if necessary
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Func {
    store: Store,
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
            $($args: WasmTy,)*
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
                $($args: WasmTy,)*
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

macro_rules! getters {
    ($(
        $(#[$doc:meta])*
        ($name:ident $(,$args:ident)*)
    )*) => ($(
        $(#[$doc])*
        #[allow(non_snake_case)]
        pub fn $name<$($args,)* R>(&self)
            -> anyhow::Result<impl Fn($($args,)*) -> Result<R, Trap>>
        where
            $($args: WasmTy,)*
            R: WasmTy,
        {
            // Verify all the paramers match the expected parameters, and that
            // there are no extra parameters...
            let mut params = self.ty().params().iter().cloned();
            let n = 0;
            $(
                let n = n + 1;
                $args::matches(&mut params)
                    .with_context(|| format!("Type mismatch in argument {}", n))?;
            )*
            ensure!(params.next().is_none(), "Type mismatch: too many arguments (expected {})", n);

            // ... then do the same for the results...
            let mut results = self.ty().results().iter().cloned();
            R::matches(&mut results)
                .context("Type mismatch in return type")?;
            ensure!(results.next().is_none(), "Type mismatch: too many return values (expected 1)");

            // ... and then once we've passed the typechecks we can hand out our
            // object since our `transmute` below should be safe!
            let (address, vmctx) = match self.wasmtime_export() {
                wasmtime_runtime::Export::Function { address, vmctx, signature: _} => {
                    (*address, *vmctx)
                }
                _ => panic!("expected function export"),
            };
            Ok(move |$($args: $args),*| -> Result<R, Trap> {
                unsafe {
                    let f = mem::transmute::<
                        *const VMFunctionBody,
                        unsafe extern "C" fn(
                            *mut VMContext,
                            *mut VMContext,
                            $($args::Abi,)*
                        ) -> R::Abi,
                    >(address);
                    let mut ret = None;
                    $(let $args = $args.into_abi();)*
                    wasmtime_runtime::catch_traps(vmctx, || {
                        ret = Some(f(vmctx, ptr::null_mut(), $($args,)*));
                    }).map_err(Trap::from_jit)?;
                    Ok(R::from_abi(vmctx, ret.unwrap()))
                }
            })
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
        ///
        /// # Examples
        ///
        /// First up we can see how simple wasm imports can be implemented, such
        /// as a function that adds its two arguments and returns the result.
        ///
        /// ```
        /// # use wasmtime::*;
        /// # fn main() -> anyhow::Result<()> {
        /// # let store = Store::default();
        /// let add = Func::wrap2(&store, |a: i32, b: i32| a + b);
        /// let module = Module::new(
        ///     &store,
        ///     r#"
        ///         (module
        ///             (import "" "" (func $add (param i32 i32) (result i32)))
        ///             (func (export "foo") (param i32 i32) (result i32)
        ///                 local.get 0
        ///                 local.get 1
        ///                 call $add))
        ///     "#,
        /// )?;
        /// let instance = Instance::new(&module, &[add.into()])?;
        /// let foo = instance.exports()[0].func().unwrap().get2::<i32, i32, i32>()?;
        /// assert_eq!(foo(1, 2)?, 3);
        /// # Ok(())
        /// # }
        /// ```
        ///
        /// We can also do the same thing, but generate a trap if the addition
        /// overflows:
        ///
        /// ```
        /// # use wasmtime::*;
        /// # fn main() -> anyhow::Result<()> {
        /// # let store = Store::default();
        /// let add = Func::wrap2(&store, |a: i32, b: i32| {
        ///     match a.checked_add(b) {
        ///         Some(i) => Ok(i),
        ///         None => Err(Trap::new("overflow")),
        ///     }
        /// });
        /// let module = Module::new(
        ///     &store,
        ///     r#"
        ///         (module
        ///             (import "" "" (func $add (param i32 i32) (result i32)))
        ///             (func (export "foo") (param i32 i32) (result i32)
        ///                 local.get 0
        ///                 local.get 1
        ///                 call $add))
        ///     "#,
        /// )?;
        /// let instance = Instance::new(&module, &[add.into()])?;
        /// let foo = instance.exports()[0].func().unwrap().get2::<i32, i32, i32>()?;
        /// assert_eq!(foo(1, 2)?, 3);
        /// assert!(foo(i32::max_value(), 1).is_err());
        /// # Ok(())
        /// # }
        /// ```
        ///
        /// And don't forget all the wasm types are supported!
        ///
        /// ```
        /// # use wasmtime::*;
        /// # fn main() -> anyhow::Result<()> {
        /// # let store = Store::default();
        /// let debug = Func::wrap4(&store, |a: i32, b: f32, c: i64, d: f64| {
        ///     println!("a={}", a);
        ///     println!("b={}", b);
        ///     println!("c={}", c);
        ///     println!("d={}", d);
        /// });
        /// let module = Module::new(
        ///     &store,
        ///     r#"
        ///         (module
        ///             (import "" "" (func $debug (param i32 f32 i64 f64)))
        ///             (func (export "foo")
        ///                 i32.const 1
        ///                 f32.const 2
        ///                 i64.const 3
        ///                 f64.const 4
        ///                 call $debug))
        ///     "#,
        /// )?;
        /// let instance = Instance::new(&module, &[debug.into()])?;
        /// let foo = instance.exports()[0].func().unwrap().get0::<()>()?;
        /// foo()?;
        /// # Ok(())
        /// # }
        /// ```
        (wrap1, A1)

        /// Creates a new `Func` from the given Rust closure, which takes 2
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap2, A1, A2)

        /// Creates a new `Func` from the given Rust closure, which takes 3
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap3, A1, A2, A3)

        /// Creates a new `Func` from the given Rust closure, which takes 4
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap4, A1, A2, A3, A4)

        /// Creates a new `Func` from the given Rust closure, which takes 5
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap5, A1, A2, A3, A4, A5)

        /// Creates a new `Func` from the given Rust closure, which takes 6
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap6, A1, A2, A3, A4, A5, A6)

        /// Creates a new `Func` from the given Rust closure, which takes 7
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap7, A1, A2, A3, A4, A5, A6, A7)

        /// Creates a new `Func` from the given Rust closure, which takes 8
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap8, A1, A2, A3, A4, A5, A6, A7, A8)

        /// Creates a new `Func` from the given Rust closure, which takes 9
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap9, A1, A2, A3, A4, A5, A6, A7, A8, A9)

        /// Creates a new `Func` from the given Rust closure, which takes 10
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap10, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10)

        /// Creates a new `Func` from the given Rust closure, which takes 11
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap11, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11)

        /// Creates a new `Func` from the given Rust closure, which takes 12
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap12, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12)

        /// Creates a new `Func` from the given Rust closure, which takes 13
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap13, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13)

        /// Creates a new `Func` from the given Rust closure, which takes 14
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap14, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, A14)

        /// Creates a new `Func` from the given Rust closure, which takes 15
        /// arguments.
        ///
        /// For more information about this function, see [`Func::wrap1`].
        (wrap15, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, A14, A15)
    }

    fn from_wrapped(
        store: &Store,
        ty: FuncType,
        callable: Rc<dyn WrappedCallable + 'static>,
    ) -> Func {
        Func {
            store: store.clone(),
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
        for param in params {
            if !param.comes_from_same_store(&self.store) {
                return Err(Trap::new(
                    "cross-`Store` values are not currently supported",
                ));
            }
        }
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

    getters! {
        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get0)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// This function serves as an optimized version of the [`Func::call`]
        /// method if the type signature of a function is statically known to
        /// the program. This method is faster than `call` on a few metrics:
        ///
        /// * Runtime type-checking only happens once, when this method is
        ///   called.
        /// * The result values, if any, aren't boxed into a vector.
        /// * Arguments and return values don't go through boxing and unboxing.
        /// * No trampolines are used to transfer control flow to/from JIT code,
        ///   instead this function jumps directly into JIT code.
        ///
        /// For more information about which Rust types match up to which wasm
        /// types, see the documentation on [`Func::wrap1`].
        ///
        /// # Return
        ///
        /// This function will return `None` if the type signature asserted
        /// statically does not match the runtime type signature. `Some`,
        /// however, will be returned if the underlying function takes one
        /// parameter of type `A` and returns the parameter `R`. Currently `R`
        /// can either be `()` (no return values) or one wasm type. At this time
        /// a multi-value return isn't supported.
        ///
        /// The returned closure will always return a `Result<R, Trap>` and an
        /// `Err` is returned if a trap happens while the wasm is executing.
        (get1, A1)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get2, A1, A2)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get3, A1, A2, A3)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get4, A1, A2, A3, A4)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get5, A1, A2, A3, A4, A5)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get6, A1, A2, A3, A4, A5, A6)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get7, A1, A2, A3, A4, A5, A6, A7)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get8, A1, A2, A3, A4, A5, A6, A7, A8)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get9, A1, A2, A3, A4, A5, A6, A7, A8, A9)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get10, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get11, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get12, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get13, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get14, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, A14)

        /// Extracts a natively-callable object from this `Func`, if the
        /// signature matches.
        ///
        /// See the [`Func::get1`] method for more documentation.
        (get15, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, A14, A15)
    }

    pub(crate) fn store(&self) -> &Store {
        &self.store
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
pub trait WasmTy {
    #[doc(hidden)]
    type Abi: Copy;
    #[doc(hidden)]
    fn push(dst: &mut Vec<ValType>);
    #[doc(hidden)]
    fn matches(tys: impl Iterator<Item = ValType>) -> anyhow::Result<()>;
    #[doc(hidden)]
    fn from_abi(vmctx: *mut VMContext, abi: Self::Abi) -> Self;
    #[doc(hidden)]
    fn into_abi(self) -> Self::Abi;
}

impl WasmTy for () {
    type Abi = ();
    fn push(_dst: &mut Vec<ValType>) {}
    fn matches(_tys: impl Iterator<Item = ValType>) -> anyhow::Result<()> {
        Ok(())
    }
    #[inline]
    fn from_abi(_vmctx: *mut VMContext, abi: Self::Abi) -> Self {
        abi
    }
    #[inline]
    fn into_abi(self) -> Self::Abi {
        self
    }
}

impl WasmTy for i32 {
    type Abi = Self;
    fn push(dst: &mut Vec<ValType>) {
        dst.push(ValType::I32);
    }
    fn matches(mut tys: impl Iterator<Item = ValType>) -> anyhow::Result<()> {
        let next = tys.next();
        ensure!(
            next == Some(ValType::I32),
            "Type mismatch, expected i32, got {:?}",
            next
        );
        Ok(())
    }
    #[inline]
    fn from_abi(_vmctx: *mut VMContext, abi: Self::Abi) -> Self {
        abi
    }
    #[inline]
    fn into_abi(self) -> Self::Abi {
        self
    }
}

impl WasmTy for i64 {
    type Abi = Self;
    fn push(dst: &mut Vec<ValType>) {
        dst.push(ValType::I64);
    }
    fn matches(mut tys: impl Iterator<Item = ValType>) -> anyhow::Result<()> {
        let next = tys.next();
        ensure!(
            next == Some(ValType::I64),
            "Type mismatch, expected i64, got {:?}",
            next
        );
        Ok(())
    }
    #[inline]
    fn from_abi(_vmctx: *mut VMContext, abi: Self::Abi) -> Self {
        abi
    }
    #[inline]
    fn into_abi(self) -> Self::Abi {
        self
    }
}

impl WasmTy for f32 {
    type Abi = Self;
    fn push(dst: &mut Vec<ValType>) {
        dst.push(ValType::F32);
    }
    fn matches(mut tys: impl Iterator<Item = ValType>) -> anyhow::Result<()> {
        let next = tys.next();
        ensure!(
            next == Some(ValType::F32),
            "Type mismatch, expected f32, got {:?}",
            next
        );
        Ok(())
    }
    #[inline]
    fn from_abi(_vmctx: *mut VMContext, abi: Self::Abi) -> Self {
        abi
    }
    #[inline]
    fn into_abi(self) -> Self::Abi {
        self
    }
}

impl WasmTy for f64 {
    type Abi = Self;
    fn push(dst: &mut Vec<ValType>) {
        dst.push(ValType::F64);
    }
    fn matches(mut tys: impl Iterator<Item = ValType>) -> anyhow::Result<()> {
        let next = tys.next();
        ensure!(
            next == Some(ValType::F64),
            "Type mismatch, expected f64, got {:?}",
            next
        );
        Ok(())
    }
    #[inline]
    fn from_abi(_vmctx: *mut VMContext, abi: Self::Abi) -> Self {
        abi
    }
    #[inline]
    fn into_abi(self) -> Self::Abi {
        self
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
    fn matches(tys: impl Iterator<Item = ValType>) -> anyhow::Result<()>;
    #[doc(hidden)]
    fn into_abi(self) -> Self::Abi;
}

impl<T: WasmTy> WasmRet for T {
    type Abi = T::Abi;
    fn push(dst: &mut Vec<ValType>) {
        T::push(dst)
    }

    fn matches(tys: impl Iterator<Item = ValType>) -> anyhow::Result<()> {
        T::matches(tys)
    }

    #[inline]
    fn into_abi(self) -> Self::Abi {
        T::into_abi(self)
    }
}

impl<T: WasmTy> WasmRet for Result<T, Trap> {
    type Abi = T::Abi;
    fn push(dst: &mut Vec<ValType>) {
        T::push(dst)
    }

    fn matches(tys: impl Iterator<Item = ValType>) -> anyhow::Result<()> {
        T::matches(tys)
    }

    #[inline]
    fn into_abi(self) -> Self::Abi {
        match self {
            Ok(val) => return val.into_abi(),
            Err(trap) => handle_trap(trap),
        }

        fn handle_trap(trap: Trap) -> ! {
            unsafe { wasmtime_runtime::raise_user_trap(Box::new(trap)) }
        }
    }
}
