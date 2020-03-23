use crate::{Extern, FuncType, Memory, Store, Trap, Val, ValType};
use anyhow::{ensure, Context as _};
use std::cmp::max;
use std::fmt;
use std::mem;
use std::panic::{self, AssertUnwindSafe};
use std::ptr;
use wasmtime_runtime::{Export, InstanceHandle, VMContext, VMFunctionBody};
use wasmtime_runtime::{ExportFunction, VMTrampoline};

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
/// You can also use the [`wrap` function](Func::wrap) to create a
/// `Func`
///
/// ```
/// # use wasmtime::*;
/// # fn main() -> anyhow::Result<()> {
/// let store = Store::default();
///
/// // Create a custom `Func` which can execute arbitrary code inside of the
/// // closure.
/// let add = Func::wrap(&store, |a: i32, b: i32| -> i32 { a + b });
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
/// # fn main() -> anyhow::Result<()> {
/// let store = Store::default();
///
/// // Here we need to define the type signature of our `Double` function and
/// // then wrap it up in a `Func`
/// let double_type = wasmtime::FuncType::new(
///     Box::new([wasmtime::ValType::I32]),
///     Box::new([wasmtime::ValType::I32])
/// );
/// let double = Func::new(&store, double_type, |_, params, results| {
///     let mut value = params[0].unwrap_i32();
///     value *= 2;
///     results[0] = value.into();
///     Ok(())
/// });
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
    instance: InstanceHandle,
    export: ExportFunction,
    ty: FuncType,
    trampoline: VMTrampoline,
}

macro_rules! getters {
    ($(
        $(#[$doc:meta])*
        ($name:ident $(,$args:ident)*)
    )*) => ($(
        $(#[$doc])*
        #[allow(non_snake_case)]
        pub fn $name<'a, $($args,)* R>(&'a self)
            -> anyhow::Result<impl Fn($($args,)*) -> Result<R, Trap> + 'a>
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
            let f = self.wasmtime_function();
            Ok(move |$($args: $args),*| -> Result<R, Trap> {
                unsafe {
                    let fnptr = mem::transmute::<
                        *const VMFunctionBody,
                        unsafe extern "C" fn(
                            *mut VMContext,
                            *mut VMContext,
                            $($args,)*
                        ) -> R,
                    >(f.address);
                    let mut ret = None;
                    $(let $args = $args.into_abi();)*
                    wasmtime_runtime::catch_traps(f.vmctx, || {
                        ret = Some(fnptr(f.vmctx, ptr::null_mut(), $($args,)*));
                    }).map_err(Trap::from_jit)?;
                    Ok(ret.unwrap())
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
    /// * `func` - the native code invoked whenever this `Func` will be called.
    ///   This closure is provided a [`Caller`] as its first argument to learn
    ///   information about the caller, and then it's passed a list of
    ///   parameters as a slice along with a mutable slice of where to write
    ///   results.
    ///
    /// Note that the implementation of `func` must adhere to the `ty`
    /// signature given, error or traps may occur if it does not respect the
    /// `ty` signature.
    ///
    /// Additionally note that this is quite a dynamic function since signatures
    /// are not statically known. For a more performant `Func` it's recommended
    /// to use [`Func::wrap`] if you can because with statically known
    /// signatures the engine can optimize the implementation much more.
    pub fn new(
        store: &Store,
        ty: FuncType,
        func: impl Fn(Caller<'_>, &[Val], &mut [Val]) -> Result<(), Trap> + 'static,
    ) -> Self {
        let store_clone = store.clone();
        let ty_clone = ty.clone();

        // Create our actual trampoline function which translates from a bunch
        // of bit patterns on the stack to actual instances of `Val` being
        // passed to the given function.
        let func = Box::new(move |caller_vmctx, values_vec: *mut u128| {
            // We have a dynamic guarantee that `values_vec` has the right
            // number of arguments and the right types of arguments. As a result
            // we should be able to safely run through them all and read them.
            let mut args = Vec::with_capacity(ty_clone.params().len());
            for (i, ty) in ty_clone.params().iter().enumerate() {
                unsafe {
                    args.push(Val::read_value_from(values_vec.add(i), ty));
                }
            }
            let mut returns = vec![Val::null(); ty_clone.results().len()];
            func(
                Caller {
                    store: &store_clone,
                    caller_vmctx,
                },
                &args,
                &mut returns,
            )?;

            // Unlike our arguments we need to dynamically check that the return
            // values produced are correct. There could be a bug in `func` that
            // produces the wrong number or wrong types of values, and we need
            // to catch that here.
            for (i, (ret, ty)) in returns.iter_mut().zip(ty_clone.results()).enumerate() {
                if ret.ty() != *ty {
                    return Err(Trap::new(
                        "function attempted to return an incompatible value",
                    ));
                }
                unsafe {
                    ret.write_value_to(values_vec.add(i));
                }
            }
            Ok(())
        });
        let (instance, export, trampoline) =
            crate::trampoline::generate_func_export(&ty, func, store).expect("generated func");
        Func {
            store: store.clone(),
            ty,
            instance,
            export,
            trampoline,
        }
    }

    /// Creates a new `Func` from the given Rust closure.
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
    /// At this time multi-value returns are not supported, and supporting this
    /// is the subject of [#1178].
    ///
    /// [#1178]: https://github.com/bytecodealliance/wasmtime/issues/1178
    ///
    /// Finally you can also optionally take [`Caller`] as the first argument of
    /// your closure. If inserted then you're able to inspect the caller's
    /// state, for example the [`Memory`] it has exported so you can read what
    /// pointers point to.
    ///
    /// Note that when using this API, the intention is to create as thin of a
    /// layer as possible for when WebAssembly calls the function provided. With
    /// sufficient inlining and optimization the WebAssembly will call straight
    /// into `func` provided, with no extra fluff entailed.
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
    /// let add = Func::wrap(&store, |a: i32, b: i32| a + b);
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
    /// let add = Func::wrap(&store, |a: i32, b: i32| {
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
    /// let debug = Func::wrap(&store, |a: i32, b: f32, c: i64, d: f64| {
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
    ///
    /// Finally if you want to get really fancy you can also implement
    /// imports that read/write wasm module's memory
    ///
    /// ```
    /// use std::str;
    ///
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let store = Store::default();
    /// let log_str = Func::wrap(&store, |caller: Caller<'_>, ptr: i32, len: i32| {
    ///     let mem = match caller.get_export("memory") {
    ///         Some(Extern::Memory(mem)) => mem,
    ///         _ => return Err(Trap::new("failed to find host memory")),
    ///     };
    ///
    ///     // We're reading raw wasm memory here so we need `unsafe`. Note
    ///     // though that this should be safe because we don't reenter wasm
    ///     // while we're reading wasm memory, nor should we clash with
    ///     // any other memory accessors (assuming they're well-behaved
    ///     // too).
    ///     unsafe {
    ///         let data = mem.data_unchecked()
    ///             .get(ptr as u32 as usize..)
    ///             .and_then(|arr| arr.get(..len as u32 as usize));
    ///         let string = match data {
    ///             Some(data) => match str::from_utf8(data) {
    ///                 Ok(s) => s,
    ///                 Err(_) => return Err(Trap::new("invalid utf-8")),
    ///             },
    ///             None => return Err(Trap::new("pointer/length out of bounds")),
    ///         };
    ///         assert_eq!(string, "Hello, world!");
    ///         println!("{}", string);
    ///     }
    ///     Ok(())
    /// });
    /// let module = Module::new(
    ///     &store,
    ///     r#"
    ///         (module
    ///             (import "" "" (func $log_str (param i32 i32)))
    ///             (func (export "foo")
    ///                 i32.const 4   ;; ptr
    ///                 i32.const 13  ;; len
    ///                 call $log_str)
    ///             (memory (export "memory") 1)
    ///             (data (i32.const 4) "Hello, world!"))
    ///     "#,
    /// )?;
    /// let instance = Instance::new(&module, &[log_str.into()])?;
    /// let foo = instance.exports()[0].func().unwrap().get0::<()>()?;
    /// foo()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn wrap<Params, Results>(store: &Store, func: impl IntoFunc<Params, Results>) -> Func {
        func.into_func(store)
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
        // We need to perform a dynamic check that the arguments given to us
        // match the signature of this function and are appropriate to pass to
        // this function. This involves checking to make sure we have the right
        // number and types of arguments as well as making sure everything is
        // from the same `Store`.
        if self.ty.params().len() != params.len() {
            return Err(Trap::new(format!(
                "expected {} arguments, got {}",
                self.ty.params().len(),
                params.len()
            )));
        }

        let mut values_vec = vec![0; max(params.len(), self.ty.results().len())];

        // Store the argument values into `values_vec`.
        let param_tys = self.ty.params().iter();
        for ((arg, slot), ty) in params.iter().zip(&mut values_vec).zip(param_tys) {
            if arg.ty() != *ty {
                return Err(Trap::new("argument type mismatch"));
            }
            if !arg.comes_from_same_store(&self.store) {
                return Err(Trap::new(
                    "cross-`Store` values are not currently supported",
                ));
            }
            unsafe {
                arg.write_value_to(slot);
            }
        }

        // Call the trampoline.
        if let Err(error) = unsafe {
            wasmtime_runtime::catch_traps(self.export.vmctx, || {
                (self.trampoline)(
                    self.export.vmctx,
                    ptr::null_mut(),
                    self.export.address,
                    values_vec.as_mut_ptr(),
                )
            })
        } {
            return Err(Trap::from_jit(error));
        }

        // Load the return values out of `values_vec`.
        let mut results = Vec::with_capacity(self.ty.results().len());
        for (index, ty) in self.ty.results().iter().enumerate() {
            unsafe {
                let ptr = values_vec.as_ptr().add(index);
                results.push(Val::read_value_from(ptr, ty));
            }
        }

        Ok(results.into())
    }

    pub(crate) fn wasmtime_function(&self) -> &wasmtime_runtime::ExportFunction {
        &self.export
    }

    pub(crate) fn from_wasmtime_function(
        export: wasmtime_runtime::ExportFunction,
        store: &Store,
        instance: InstanceHandle,
    ) -> Self {
        // Signatures should always be registered in the store's registry of
        // shared signatures, so we should be able to unwrap safely here.
        let sig = store
            .compiler()
            .signatures()
            .lookup(export.signature)
            .expect("failed to lookup signature");

        // This is only called with `Export::Function`, and since it's coming
        // from wasmtime_runtime itself we should support all the types coming
        // out of it, so assert such here.
        let ty = FuncType::from_wasmtime_signature(sig)
            .expect("core wasm signature should be supported");

        // Each function signature in a module should have a trampoline stored
        // on that module as well, so unwrap the result here since otherwise
        // it's a bug in wasmtime.
        let trampoline = instance
            .trampoline(export.signature)
            .expect("failed to retrieve trampoline from module");

        Func {
            instance,
            export,
            trampoline,
            ty,
            store: store.clone(),
        }
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
        /// types, see the documentation on [`Func::wrap`].
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
/// [`Func::wrap`] and friends.
///
/// This trait should not be implemented by user types. This trait may change at
/// any time internally. The types which implement this trait, however, are
/// stable over time.
///
/// For more information see [`Func::wrap`]
pub unsafe trait WasmTy: Copy {
    #[doc(hidden)]
    fn push(dst: &mut Vec<ValType>);
    #[doc(hidden)]
    fn matches(tys: impl Iterator<Item = ValType>) -> anyhow::Result<()>;
    #[doc(hidden)]
    unsafe fn load(ptr: &mut *const u128) -> Self;
    #[doc(hidden)]
    unsafe fn store(abi: Self, ptr: *mut u128);
}

unsafe impl WasmTy for () {
    fn push(_dst: &mut Vec<ValType>) {}
    fn matches(_tys: impl Iterator<Item = ValType>) -> anyhow::Result<()> {
        Ok(())
    }
    #[inline]
    unsafe fn load(_ptr: &mut *const u128) -> Self {}
    #[inline]
    unsafe fn store(_abi: Self, _ptr: *mut u128) {}
}

unsafe impl WasmTy for i32 {
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
    unsafe fn load(ptr: &mut *const u128) -> Self {
        let ret = **ptr as Self;
        *ptr = (*ptr).add(1);
        return ret;
    }
    #[inline]
    unsafe fn store(abi: Self, ptr: *mut u128) {
        *ptr = abi as u128;
    }
}

unsafe impl WasmTy for i64 {
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
    unsafe fn load(ptr: &mut *const u128) -> Self {
        let ret = **ptr as Self;
        *ptr = (*ptr).add(1);
        return ret;
    }
    #[inline]
    unsafe fn store(abi: Self, ptr: *mut u128) {
        *ptr = abi as u128;
    }
}

unsafe impl WasmTy for f32 {
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
    unsafe fn load(ptr: &mut *const u128) -> Self {
        let ret = f32::from_bits(**ptr as u32);
        *ptr = (*ptr).add(1);
        return ret;
    }
    #[inline]
    unsafe fn store(abi: Self, ptr: *mut u128) {
        *ptr = abi.to_bits() as u128;
    }
}

unsafe impl WasmTy for f64 {
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
    unsafe fn load(ptr: &mut *const u128) -> Self {
        let ret = f64::from_bits(**ptr as u64);
        *ptr = (*ptr).add(1);
        return ret;
    }
    #[inline]
    unsafe fn store(abi: Self, ptr: *mut u128) {
        *ptr = abi.to_bits() as u128;
    }
}

/// A trait implemented for types which can be returned from closures passed to
/// [`Func::wrap`] and friends.
///
/// This trait should not be implemented by user types. This trait may change at
/// any time internally. The types which implement this trait, however, are
/// stable over time.
///
/// For more information see [`Func::wrap`]
pub unsafe trait WasmRet {
    #[doc(hidden)]
    type Abi;
    #[doc(hidden)]
    fn push(dst: &mut Vec<ValType>);
    #[doc(hidden)]
    fn matches(tys: impl Iterator<Item = ValType>) -> anyhow::Result<()>;
    #[doc(hidden)]
    fn into_abi(self) -> Self::Abi;
    #[doc(hidden)]
    unsafe fn store(abi: Self::Abi, ptr: *mut u128);
}

unsafe impl<T: WasmTy> WasmRet for T {
    type Abi = T;
    fn push(dst: &mut Vec<ValType>) {
        T::push(dst)
    }

    fn matches(tys: impl Iterator<Item = ValType>) -> anyhow::Result<()> {
        T::matches(tys)
    }

    #[inline]
    fn into_abi(self) -> Self::Abi {
        self
    }

    #[inline]
    unsafe fn store(abi: Self::Abi, ptr: *mut u128) {
        T::store(abi, ptr);
    }
}

unsafe impl<T: WasmTy> WasmRet for Result<T, Trap> {
    type Abi = T;
    fn push(dst: &mut Vec<ValType>) {
        T::push(dst)
    }

    fn matches(tys: impl Iterator<Item = ValType>) -> anyhow::Result<()> {
        T::matches(tys)
    }

    #[inline]
    fn into_abi(self) -> Self::Abi {
        match self {
            Ok(val) => return T::into_abi(val),
            Err(trap) => handle_trap(trap),
        }

        fn handle_trap(trap: Trap) -> ! {
            unsafe { wasmtime_runtime::raise_user_trap(Box::new(trap)) }
        }
    }

    #[inline]
    unsafe fn store(abi: Self::Abi, ptr: *mut u128) {
        T::store(abi, ptr);
    }
}

/// Internal trait implemented for all arguments that can be passed to
/// [`Func::wrap`].
///
/// This trait should not be implemented by external users, it's only intended
/// as an implementation detail of this crate.
pub trait IntoFunc<Params, Results> {
    #[doc(hidden)]
    fn into_func(self, store: &Store) -> Func;
}

/// A structure representing the *caller's* context when creating a function
/// via [`Func::wrap`].
///
/// This structure can be taken as the first parameter of a closure passed to
/// [`Func::wrap`], and it can be used to learn information about the caller of
/// the function, such as the calling module's memory, exports, etc.
///
/// The primary purpose of this structure is to provide access to the
/// caller's information, such as it's exported memory. This allows
/// functions which take pointers as arguments to easily read the memory the
/// pointers point into.
///
/// Note that this is intended to be a pretty temporary mechanism for accessing
/// the caller's memory until interface types has been fully standardized and
/// implemented.
pub struct Caller<'a> {
    store: &'a Store,
    caller_vmctx: *mut VMContext,
}

impl Caller<'_> {
    /// Looks up an export from the caller's module by the `name` given.
    ///
    /// Note that this function is only implemented for the `Extern::Memory`
    /// type currently. No other exported structure can be acquired through this
    /// just yet, but this may be implemented in the future!
    ///
    /// # Return
    ///
    /// If a memory export with the `name` provided was found, then it is
    /// returned as a `Memory`. There are a number of situations, however, where
    /// the memory may not be available:
    ///
    /// * The caller instance may not have an export named `name`
    /// * The export named `name` may not be an exported memory
    /// * There may not be a caller available, for example if `Func` was called
    ///   directly from host code.
    ///
    /// It's recommended to take care when calling this API and gracefully
    /// handling a `None` return value.
    pub fn get_export(&self, name: &str) -> Option<Extern> {
        unsafe {
            if self.caller_vmctx.is_null() {
                return None;
            }
            let instance = InstanceHandle::from_vmctx(self.caller_vmctx);
            let export = match instance.lookup(name) {
                Some(Export::Memory(m)) => m,
                _ => return None,
            };
            let mem = Memory::from_wasmtime_memory(export, self.store, instance);
            Some(Extern::Memory(mem))
        }
    }
}

macro_rules! impl_into_func {
    ($(
        ($($args:ident)*)
    )*) => ($(
        // Implement for functions without a leading `&Caller` parameter,
        // delegating to the implementation below which does have the leading
        // `Caller` parameter.
        impl<F, $($args,)* R> IntoFunc<($($args,)*), R> for F
        where
            F: Fn($($args),*) -> R + 'static,
            $($args: WasmTy,)*
            R: WasmRet,
        {
            #[allow(non_snake_case)]
            fn into_func(self, store: &Store) -> Func {
                Func::wrap(store, move |_: Caller<'_>, $($args:$args),*| {
                    self($($args),*)
                })
            }
        }

        #[allow(non_snake_case)]
        impl<F, $($args,)* R> IntoFunc<(Caller<'_>, $($args,)*), R> for F
        where
            F: Fn(Caller<'_>, $($args),*) -> R + 'static,
            $($args: WasmTy,)*
            R: WasmRet,
        {
            fn into_func(self, store: &Store) -> Func {
                // Note that this shim's ABI must match that expected by
                // cranelift, since cranelift is generating raw function calls
                // directly to this function.
                unsafe extern "C" fn shim<F, $($args,)* R>(
                    vmctx: *mut VMContext,
                    caller_vmctx: *mut VMContext,
                    $($args: $args,)*
                ) -> R::Abi
                where
                    F: Fn(Caller<'_>, $($args),*) -> R + 'static,
                    $($args: WasmTy,)*
                    R: WasmRet,
                {
                    let ret = {
                        let instance = InstanceHandle::from_vmctx(vmctx);
                        let (func, store) = instance.host_state().downcast_ref::<(F, Store)>().expect("state");
                        panic::catch_unwind(AssertUnwindSafe(|| {
                            func(
                                Caller { store, caller_vmctx },
                                $($args,)*
                            )
                        }))
                    };
                    match ret {
                        Ok(ret) => ret.into_abi(),
                        Err(panic) => wasmtime_runtime::resume_panic(panic),
                    }
                }

                unsafe extern "C" fn trampoline<$($args,)* R>(
                    callee_vmctx: *mut VMContext,
                    caller_vmctx: *mut VMContext,
                    ptr: *const VMFunctionBody,
                    args: *mut u128,
                )
                where
                    $($args: WasmTy,)*
                    R: WasmRet,
                {
                    let ptr = mem::transmute::<
                        *const VMFunctionBody,
                        unsafe extern "C" fn(
                            *mut VMContext,
                            *mut VMContext,
                            $($args,)*
                        ) -> R::Abi,
                    >(ptr);

                    let mut _next = args as *const u128;
                    $(let $args = $args::load(&mut _next);)*
                    let ret = ptr(callee_vmctx, caller_vmctx, $($args),*);
                    R::store(ret, args);
                }

                let mut _args = Vec::new();
                $($args::push(&mut _args);)*
                let mut ret = Vec::new();
                R::push(&mut ret);
                let ty = FuncType::new(_args.into(), ret.into());
                let store_clone = store.clone();
                unsafe {
					let trampoline = trampoline::<$($args,)* R>;
                    let (instance, export) = crate::trampoline::generate_raw_func_export(
                        &ty,
                        std::slice::from_raw_parts_mut(
                            shim::<F, $($args,)* R> as *mut _,
                            0,
                        ),
                        trampoline,
                        store,
                        Box::new((self, store_clone)),
                    )
                    .expect("failed to generate export");
                    Func {
                        store: store.clone(),
                        ty,
                        instance,
                        export,
                        trampoline,
                    }
                }
            }
        }
    )*)
}

impl_into_func! {
    ()
    (A1)
    (A1 A2)
    (A1 A2 A3)
    (A1 A2 A3 A4)
    (A1 A2 A3 A4 A5)
    (A1 A2 A3 A4 A5 A6)
    (A1 A2 A3 A4 A5 A6 A7)
    (A1 A2 A3 A4 A5 A6 A7 A8)
    (A1 A2 A3 A4 A5 A6 A7 A8 A9)
    (A1 A2 A3 A4 A5 A6 A7 A8 A9 A10)
    (A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11)
    (A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 A12)
    (A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 A12 A13)
    (A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 A12 A13 A14)
    (A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 A12 A13 A14 A15)
}
