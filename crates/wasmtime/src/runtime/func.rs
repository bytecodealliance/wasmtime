use crate::prelude::*;
use crate::runtime::vm::{
    ExportFunction, SendSyncPtr, StoreBox, VMArrayCallHostFuncContext, VMContext, VMFuncRef,
    VMFunctionImport, VMNativeCallHostFuncContext, VMOpaqueContext,
};
use crate::runtime::Uninhabited;
use crate::store::{AutoAssertNoGc, StoreData, StoreOpaque, Stored};
use crate::type_registry::RegisteredType;
use crate::{
    AsContext, AsContextMut, CallHook, Engine, Extern, FuncType, Instance, Module, Ref,
    StoreContext, StoreContextMut, Val, ValRaw, ValType,
};
use alloc::sync::Arc;
use anyhow::{bail, Context as _, Error, Result};
use core::ffi::c_void;
use core::future::Future;
use core::mem;
use core::num::NonZeroUsize;
use core::pin::Pin;
use core::ptr::{self, NonNull};
use wasmtime_environ::VMSharedTypeIndex;

/// A reference to the abstract `nofunc` heap value.
///
/// The are no instances of `(ref nofunc)`: it is an uninhabited type.
///
/// There is precisely one instance of `(ref null nofunc)`, aka `nullfuncref`:
/// the null reference.
///
/// This `NoFunc` Rust type's sole purpose is for use with [`Func::wrap`]- and
/// [`Func::typed`]-style APIs for statically typing a function as taking or
/// returning a `(ref null nofunc)` (aka `Option<NoFunc>`) which is always
/// `None`.
///
/// # Example
///
/// ```
/// # use wasmtime::*;
/// # fn _foo() -> Result<()> {
/// let mut config = Config::new();
/// config.wasm_function_references(true);
/// let engine = Engine::new(&config)?;
///
/// let module = Module::new(
///     &engine,
///     r#"
///         (module
///             (func (export "f") (param (ref null nofunc))
///                 ;; If the reference is null, return.
///                 local.get 0
///                 ref.is_null nofunc
///                 br_if 0
///
///                 ;; If the reference was not null (which is impossible)
///                 ;; then raise a trap.
///                 unreachable
///             )
///         )
///     "#,
/// )?;
///
/// let mut store = Store::new(&engine, ());
/// let instance = Instance::new(&mut store, &module, &[])?;
/// let f = instance.get_func(&mut store, "f").unwrap();
///
/// // We can cast a `(ref null nofunc)`-taking function into a typed function that
/// // takes an `Option<NoFunc>` via the `Func::typed` method.
/// let f = f.typed::<Option<NoFunc>, ()>(&store)?;
///
/// // We can call the typed function, passing the null `nofunc` reference.
/// let result = f.call(&mut store, NoFunc::null());
///
/// // The function should not have trapped, because the reference we gave it was
/// // null (as it had to be, since `NoFunc` is uninhabited).
/// assert!(result.is_ok());
/// # Ok(())
/// # }
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NoFunc {
    _inner: Uninhabited,
}

impl NoFunc {
    /// Get the null `(ref null nofunc)` (aka `nullfuncref`) reference.
    #[inline]
    pub fn null() -> Option<NoFunc> {
        None
    }

    /// Get the null `(ref null nofunc)` (aka `nullfuncref`) reference as a
    /// [`Ref`].
    #[inline]
    pub fn null_ref() -> Ref {
        Ref::Func(None)
    }

    /// Get the null `(ref null nofunc)` (aka `nullfuncref`) reference as a
    /// [`Val`].
    #[inline]
    pub fn null_val() -> Val {
        Val::FuncRef(None)
    }
}

/// A WebAssembly function which can be called.
///
/// This type typically represents an exported function from a WebAssembly
/// module instance. In this case a [`Func`] belongs to an [`Instance`] and is
/// loaded from there. A [`Func`] may also represent a host function as well in
/// some cases, too.
///
/// Functions can be called in a few different ways, either synchronous or async
/// and either typed or untyped (more on this below). Note that host functions
/// are normally inserted directly into a [`Linker`](crate::Linker) rather than
/// using this directly, but both options are available.
///
/// # `Func` and `async`
///
/// Functions from the perspective of WebAssembly are always synchronous. You
/// might have an `async` function in Rust, however, which you'd like to make
/// available from WebAssembly. Wasmtime supports asynchronously calling
/// WebAssembly through native stack switching. You can get some more
/// information about [asynchronous configs](crate::Config::async_support), but
/// from the perspective of `Func` it's important to know that whether or not
/// your [`Store`](crate::Store) is asynchronous will dictate whether you call
/// functions through [`Func::call`] or [`Func::call_async`] (or the typed
/// wrappers such as [`TypedFunc::call`] vs [`TypedFunc::call_async`]).
///
/// # To `Func::call` or to `Func::typed().call()`
///
/// There's a 2x2 matrix of methods to call [`Func`]. Invocations can either be
/// asynchronous or synchronous. They can also be statically typed or not.
/// Whether or not an invocation is asynchronous is indicated via the method
/// being `async` and [`call_async`](Func::call_async) being the entry point.
/// Otherwise for statically typed or not your options are:
///
/// * Dynamically typed - if you don't statically know the signature of the
///   function that you're calling you'll be using [`Func::call`] or
///   [`Func::call_async`]. These functions take a variable-length slice of
///   "boxed" arguments in their [`Val`] representation. Additionally the
///   results are returned as an owned slice of [`Val`]. These methods are not
///   optimized due to the dynamic type checks that must occur, in addition to
///   some dynamic allocations for where to put all the arguments. While this
///   allows you to call all possible wasm function signatures, if you're
///   looking for a speedier alternative you can also use...
///
/// * Statically typed - if you statically know the type signature of the wasm
///   function you're calling, then you'll want to use the [`Func::typed`]
///   method to acquire an instance of [`TypedFunc`]. This structure is static proof
///   that the underlying wasm function has the ascripted type, and type
///   validation is only done once up-front. The [`TypedFunc::call`] and
///   [`TypedFunc::call_async`] methods are much more efficient than [`Func::call`]
///   and [`Func::call_async`] because the type signature is statically known.
///   This eschews runtime checks as much as possible to get into wasm as fast
///   as possible.
///
/// # Examples
///
/// One way to get a `Func` is from an [`Instance`] after you've instantiated
/// it:
///
/// ```
/// # use wasmtime::*;
/// # fn main() -> anyhow::Result<()> {
/// let engine = Engine::default();
/// let module = Module::new(&engine, r#"(module (func (export "foo")))"#)?;
/// let mut store = Store::new(&engine, ());
/// let instance = Instance::new(&mut store, &module, &[])?;
/// let foo = instance.get_func(&mut store, "foo").expect("export wasn't a function");
///
/// // Work with `foo` as a `Func` at this point, such as calling it
/// // dynamically...
/// match foo.call(&mut store, &[], &mut []) {
///     Ok(()) => { /* ... */ }
///     Err(trap) => {
///         panic!("execution of `foo` resulted in a wasm trap: {}", trap);
///     }
/// }
/// foo.call(&mut store, &[], &mut [])?;
///
/// // ... or we can make a static assertion about its signature and call it.
/// // Our first call here can fail if the signatures don't match, and then the
/// // second call can fail if the function traps (like the `match` above).
/// let foo = foo.typed::<(), ()>(&store)?;
/// foo.call(&mut store, ())?;
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
/// let mut store = Store::<()>::default();
///
/// // Create a custom `Func` which can execute arbitrary code inside of the
/// // closure.
/// let add = Func::wrap(&mut store, |a: i32, b: i32| -> i32 { a + b });
///
/// // Next we can hook that up to a wasm module which uses it.
/// let module = Module::new(
///     store.engine(),
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
/// let instance = Instance::new(&mut store, &module, &[add.into()])?;
/// let call_add_twice = instance.get_typed_func::<(), i32>(&mut store, "call_add_twice")?;
///
/// assert_eq!(call_add_twice.call(&mut store, ())?, 10);
/// # Ok(())
/// # }
/// ```
///
/// Or you could also create an entirely dynamic `Func`!
///
/// ```
/// # use wasmtime::*;
/// # fn main() -> anyhow::Result<()> {
/// let mut store = Store::<()>::default();
///
/// // Here we need to define the type signature of our `Double` function and
/// // then wrap it up in a `Func`
/// let double_type = wasmtime::FuncType::new(
///     store.engine(),
///     [wasmtime::ValType::I32].iter().cloned(),
///     [wasmtime::ValType::I32].iter().cloned(),
/// );
/// let double = Func::new(&mut store, double_type, |_, params, results| {
///     let mut value = params[0].unwrap_i32();
///     value *= 2;
///     results[0] = value.into();
///     Ok(())
/// });
///
/// let module = Module::new(
///     store.engine(),
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
/// let instance = Instance::new(&mut store, &module, &[double.into()])?;
/// // .. work with `instance` if necessary
/// # Ok(())
/// # }
/// ```
#[derive(Copy, Clone, Debug)]
#[repr(transparent)] // here for the C API
pub struct Func(Stored<FuncData>);

pub(crate) struct FuncData {
    kind: FuncKind,

    // A pointer to the in-store `VMFuncRef` for this function, if
    // any.
    //
    // When a function is passed to Wasm but doesn't have a Wasm-to-native
    // trampoline, we have to patch it in. But that requires mutating the
    // `VMFuncRef`, and this function could be shared across
    // threads. So we instead copy and pin the `VMFuncRef` into
    // `StoreOpaque::func_refs`, where we can safely patch the field without
    // worrying about synchronization and we hold a pointer to it here so we can
    // reuse it rather than re-copy if it is passed to Wasm again.
    in_store_func_ref: Option<SendSyncPtr<VMFuncRef>>,

    // This is somewhat expensive to load from the `Engine` and in most
    // optimized use cases (e.g. `TypedFunc`) it's not actually needed or it's
    // only needed rarely. To handle that this is an optionally-contained field
    // which is lazily loaded into as part of `Func::call`.
    //
    // Also note that this is intentionally placed behind a pointer to keep it
    // small as `FuncData` instances are often inserted into a `Store`.
    ty: Option<Box<FuncType>>,
}

/// The three ways that a function can be created and referenced from within a
/// store.
enum FuncKind {
    /// A function already owned by the store via some other means. This is
    /// used, for example, when creating a `Func` from an instance's exported
    /// function. The instance's `InstanceHandle` is already owned by the store
    /// and we just have some pointers into that which represent how to call the
    /// function.
    StoreOwned { export: ExportFunction },

    /// A function is shared across possibly other stores, hence the `Arc`. This
    /// variant happens when a `Linker`-defined function is instantiated within
    /// a `Store` (e.g. via `Linker::get` or similar APIs). The `Arc` here
    /// indicates that there's some number of other stores holding this function
    /// too, so dropping this may not deallocate the underlying
    /// `InstanceHandle`.
    SharedHost(Arc<HostFunc>),

    /// A uniquely-owned host function within a `Store`. This comes about with
    /// `Func::new` or similar APIs. The `HostFunc` internally owns the
    /// `InstanceHandle` and that will get dropped when this `HostFunc` itself
    /// is dropped.
    ///
    /// Note that this is intentionally placed behind a `Box` to minimize the
    /// size of this enum since the most common variant for high-peformance
    /// situations is `SharedHost` and `StoreOwned`, so this ideally isn't
    /// larger than those two.
    Host(Box<HostFunc>),

    /// A reference to a `HostFunc`, but one that's "rooted" in the `Store`
    /// itself.
    ///
    /// This variant is created when an `InstancePre<T>` is instantiated in to a
    /// `Store<T>`. In that situation the `InstancePre<T>` already has a list of
    /// host functions that are packaged up in an `Arc`, so the `Arc<[T]>` is
    /// cloned once into the `Store` to avoid each individual function requiring
    /// an `Arc::clone`.
    ///
    /// The lifetime management of this type is `unsafe` because
    /// `RootedHostFunc` is a small wrapper around `NonNull<HostFunc>`. To be
    /// safe this is required that the memory of the host function is pinned
    /// elsewhere (e.g. the `Arc` in the `Store`).
    RootedHost(RootedHostFunc),
}

macro_rules! for_each_function_signature {
    ($mac:ident) => {
        $mac!(0);
        $mac!(1 A1);
        $mac!(2 A1 A2);
        $mac!(3 A1 A2 A3);
        $mac!(4 A1 A2 A3 A4);
        $mac!(5 A1 A2 A3 A4 A5);
        $mac!(6 A1 A2 A3 A4 A5 A6);
        $mac!(7 A1 A2 A3 A4 A5 A6 A7);
        $mac!(8 A1 A2 A3 A4 A5 A6 A7 A8);
        $mac!(9 A1 A2 A3 A4 A5 A6 A7 A8 A9);
        $mac!(10 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10);
        $mac!(11 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11);
        $mac!(12 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 A12);
        $mac!(13 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 A12 A13);
        $mac!(14 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 A12 A13 A14);
        $mac!(15 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 A12 A13 A14 A15);
        $mac!(16 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 A12 A13 A14 A15 A16);
    };
}

mod typed;
pub use typed::*;

macro_rules! generate_wrap_async_func {
    ($num:tt $($args:ident)*) => (paste::paste!{
        /// Same as [`Func::wrap`], except the closure asynchronously produces
        /// its result. For more information see the [`Func`] documentation.
        ///
        /// # Panics
        ///
        /// This function will panic if called with a non-asynchronous store.
        #[allow(non_snake_case)]
        #[cfg(feature = "async")]
        pub fn [<wrap $num _async>]<T, $($args,)* R>(
            store: impl AsContextMut<Data = T>,
            func: impl for<'a> Fn(Caller<'a, T>, $($args),*) -> Box<dyn Future<Output = R> + Send + 'a> + Send + Sync + 'static,
        ) -> Func
        where
            $($args: WasmTy,)*
            R: WasmRet,
        {
            assert!(store.as_context().async_support(), concat!("cannot use `wrap", $num, "_async` without enabling async support on the config"));
            Func::wrap(store, move |mut caller: Caller<'_, T>, $($args: $args),*| {
                let async_cx = caller.store.as_context_mut().0.async_cx().expect("Attempt to start async function on dying fiber");
                let mut future = Pin::from(func(caller, $($args),*));

                match unsafe { async_cx.block_on(future.as_mut()) } {
                    Ok(ret) => ret.into_fallible(),
                    Err(e) => R::fallible_from_error(e),
                }
            })
        }
    })
}

impl Func {
    /// Creates a new `Func` with the given arguments, typically to create a
    /// host-defined function to pass as an import to a module.
    ///
    /// * `store` - the store in which to create this [`Func`], which will own
    ///   the return value.
    ///
    /// * `ty` - the signature of this function, used to indicate what the
    ///   inputs and outputs are.
    ///
    /// * `func` - the native code invoked whenever this `Func` will be called.
    ///   This closure is provided a [`Caller`] as its first argument to learn
    ///   information about the caller, and then it's passed a list of
    ///   parameters as a slice along with a mutable slice of where to write
    ///   results.
    ///
    /// Note that the implementation of `func` must adhere to the `ty` signature
    /// given, error or traps may occur if it does not respect the `ty`
    /// signature. For example if the function type declares that it returns one
    /// i32 but the `func` closures does not write anything into the results
    /// slice then a trap may be generated.
    ///
    /// Additionally note that this is quite a dynamic function since signatures
    /// are not statically known. For a more performant and ergonomic `Func`
    /// it's recommended to use [`Func::wrap`] if you can because with
    /// statically known signatures Wasmtime can optimize the implementation
    /// much more.
    ///
    /// For more information about `Send + Sync + 'static` requirements on the
    /// `func`, see [`Func::wrap`](#why-send--sync--static).
    ///
    /// # Errors
    ///
    /// The host-provided function here returns a
    /// [`Result<()>`](anyhow::Result). If the function returns `Ok(())` then
    /// that indicates that the host function completed successfully and wrote
    /// the result into the `&mut [Val]` argument.
    ///
    /// If the function returns `Err(e)`, however, then this is equivalent to
    /// the host function triggering a trap for wasm. WebAssembly execution is
    /// immediately halted and the original caller of [`Func::call`], for
    /// example, will receive the error returned here (possibly with
    /// [`WasmBacktrace`](crate::WasmBacktrace) context information attached).
    ///
    /// For more information about errors in Wasmtime see the [`Trap`]
    /// documentation.
    ///
    /// [`Trap`]: crate::Trap
    ///
    /// # Panics
    ///
    /// Panics if the given function type is not associated with this store's
    /// engine.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn new<T>(
        store: impl AsContextMut<Data = T>,
        ty: FuncType,
        func: impl Fn(Caller<'_, T>, &[Val], &mut [Val]) -> Result<()> + Send + Sync + 'static,
    ) -> Self {
        assert!(ty.comes_from_same_engine(store.as_context().engine()));
        let ty_clone = ty.clone();
        unsafe {
            Func::new_unchecked(store, ty, move |caller, values| {
                Func::invoke_host_func_for_wasm(caller, &ty_clone, values, &func)
            })
        }
    }

    /// Creates a new [`Func`] with the given arguments, although has fewer
    /// runtime checks than [`Func::new`].
    ///
    /// This function takes a callback of a different signature than
    /// [`Func::new`], instead receiving a raw pointer with a list of [`ValRaw`]
    /// structures. These values have no type information associated with them
    /// so it's up to the caller to provide a function that will correctly
    /// interpret the list of values as those coming from the `ty` specified.
    ///
    /// If you're calling this from Rust it's recommended to either instead use
    /// [`Func::new`] or [`Func::wrap`]. The [`Func::wrap`] API, in particular,
    /// is both safer and faster than this API.
    ///
    /// # Errors
    ///
    /// See [`Func::new`] for the behavior of returning an error from the host
    /// function provided here.
    ///
    /// # Unsafety
    ///
    /// This function is not safe because it's not known at compile time that
    /// the `func` provided correctly interprets the argument types provided to
    /// it, or that the results it produces will be of the correct type.
    ///
    /// # Panics
    ///
    /// Panics if the given function type is not associated with this store's
    /// engine.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub unsafe fn new_unchecked<T>(
        mut store: impl AsContextMut<Data = T>,
        ty: FuncType,
        func: impl Fn(Caller<'_, T>, &mut [ValRaw]) -> Result<()> + Send + Sync + 'static,
    ) -> Self {
        assert!(ty.comes_from_same_engine(store.as_context().engine()));
        let store = store.as_context_mut().0;
        let host = HostFunc::new_unchecked(store.engine(), ty, func);
        host.into_func(store)
    }

    /// Creates a new host-defined WebAssembly function which, when called,
    /// will run the asynchronous computation defined by `func` to completion
    /// and then return the result to WebAssembly.
    ///
    /// This function is the asynchronous analogue of [`Func::new`] and much of
    /// that documentation applies to this as well. The key difference is that
    /// `func` returns a future instead of simply a `Result`. Note that the
    /// returned future can close over any of the arguments, but it cannot close
    /// over the state of the closure itself. It's recommended to store any
    /// necessary async state in the `T` of the [`Store<T>`](crate::Store) which
    /// can be accessed through [`Caller::data`] or [`Caller::data_mut`].
    ///
    /// For more information on `Send + Sync + 'static`, see
    /// [`Func::wrap`](#why-send--sync--static).
    ///
    /// # Panics
    ///
    /// This function will panic if `store` is not associated with an [async
    /// config](crate::Config::async_support).
    ///
    /// Panics if the given function type is not associated with this store's
    /// engine.
    ///
    /// # Errors
    ///
    /// See [`Func::new`] for the behavior of returning an error from the host
    /// function provided here.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// // Simulate some application-specific state as well as asynchronous
    /// // functions to query that state.
    /// struct MyDatabase {
    ///     // ...
    /// }
    ///
    /// impl MyDatabase {
    ///     async fn get_row_count(&self) -> u32 {
    ///         // ...
    /// #       100
    ///     }
    /// }
    ///
    /// let my_database = MyDatabase {
    ///     // ...
    /// };
    ///
    /// // Using `new_async` we can hook up into calling our async
    /// // `get_row_count` function.
    /// let engine = Engine::new(Config::new().async_support(true))?;
    /// let mut store = Store::new(&engine, MyDatabase {
    ///     // ...
    /// });
    /// let get_row_count_type = wasmtime::FuncType::new(
    ///     &engine,
    ///     None,
    ///     Some(wasmtime::ValType::I32),
    /// );
    /// let get = Func::new_async(&mut store, get_row_count_type, |caller, _params, results| {
    ///     Box::new(async move {
    ///         let count = caller.data().get_row_count().await;
    ///         results[0] = Val::I32(count as i32);
    ///         Ok(())
    ///     })
    /// });
    /// // ...
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(all(feature = "async", feature = "cranelift"))]
    pub fn new_async<T, F>(store: impl AsContextMut<Data = T>, ty: FuncType, func: F) -> Func
    where
        F: for<'a> Fn(
                Caller<'a, T>,
                &'a [Val],
                &'a mut [Val],
            ) -> Box<dyn Future<Output = Result<()>> + Send + 'a>
            + Send
            + Sync
            + 'static,
    {
        assert!(
            store.as_context().async_support(),
            "cannot use `new_async` without enabling async support in the config"
        );
        assert!(ty.comes_from_same_engine(store.as_context().engine()));
        Func::new(store, ty, move |mut caller, params, results| {
            let async_cx = caller
                .store
                .as_context_mut()
                .0
                .async_cx()
                .expect("Attempt to spawn new action on dying fiber");
            let mut future = Pin::from(func(caller, params, results));
            match unsafe { async_cx.block_on(future.as_mut()) } {
                Ok(Ok(())) => Ok(()),
                Ok(Err(trap)) | Err(trap) => Err(trap),
            }
        })
    }

    pub(crate) unsafe fn from_vm_func_ref(
        store: &mut StoreOpaque,
        raw: *mut VMFuncRef,
    ) -> Option<Func> {
        let func_ref = NonNull::new(raw)?;
        debug_assert!(func_ref.as_ref().type_index != VMSharedTypeIndex::default());
        let export = ExportFunction { func_ref };
        Some(Func::from_wasmtime_function(export, store))
    }

    /// Creates a new `Func` from the given Rust closure.
    ///
    /// This function will create a new `Func` which, when called, will
    /// execute the given Rust closure. Unlike [`Func::new`] the target
    /// function being called is known statically so the type signature can
    /// be inferred. Rust types will map to WebAssembly types as follows:
    ///
    /// | Rust Argument Type                | WebAssembly Type                          |
    /// |-----------------------------------|-------------------------------------------|
    /// | `i32`                             | `i32`                                     |
    /// | `u32`                             | `i32`                                     |
    /// | `i64`                             | `i64`                                     |
    /// | `u64`                             | `i64`                                     |
    /// | `f32`                             | `f32`                                     |
    /// | `f64`                             | `f64`                                     |
    /// | `V128` on x86-64 and aarch64 only | `v128`                                    |
    /// | `Option<Func>`                    | `funcref` aka `(ref null func)`           |
    /// | `Func`                            | `(ref func)`                              |
    /// | `Option<Nofunc>`                  | `nullfuncref` aka `(ref null nofunc)`     |
    /// | `NoFunc`                          | `(ref nofunc)`                            |
    /// | `Option<ExternRef>`               | `externref` aka `(ref null extern)`       |
    /// | `ExternRef`                       | `(ref extern)`                            |
    /// | `Option<NoExtern>`                | `nullexternref` aka `(ref null noextern)` |
    /// | `NoExtern`                        | `(ref noextern)`                          |
    /// | `Option<AnyRef>`                  | `anyref` aka `(ref null any)`             |
    /// | `AnyRef`                          | `(ref any)`                               |
    /// | `Option<I31>`                     | `i31ref` aka `(ref null i31)`             |
    /// | `I31`                             | `(ref i31)`                               |
    ///
    /// Any of the Rust types can be returned from the closure as well, in
    /// addition to some extra types
    ///
    /// | Rust Return Type  | WebAssembly Return Type | Meaning               |
    /// |-------------------|-------------------------|-----------------------|
    /// | `()`              | nothing                 | no return value       |
    /// | `T`               | `T`                     | a single return value |
    /// | `(T1, T2, ...)`   | `T1 T2 ...`             | multiple returns      |
    ///
    /// Note that all return types can also be wrapped in `Result<_>` to
    /// indicate that the host function can generate a trap as well as possibly
    /// returning a value.
    ///
    /// Finally you can also optionally take [`Caller`] as the first argument of
    /// your closure. If inserted then you're able to inspect the caller's
    /// state, for example the [`Memory`](crate::Memory) it has exported so you
    /// can read what pointers point to.
    ///
    /// Note that when using this API, the intention is to create as thin of a
    /// layer as possible for when WebAssembly calls the function provided. With
    /// sufficient inlining and optimization the WebAssembly will call straight
    /// into `func` provided, with no extra fluff entailed.
    ///
    /// # Why `Send + Sync + 'static`?
    ///
    /// All host functions defined in a [`Store`](crate::Store) (including
    /// those from [`Func::new`] and other constructors) require that the
    /// `func` provided is `Send + Sync + 'static`. Additionally host functions
    /// always are `Fn` as opposed to `FnMut` or `FnOnce`. This can at-a-glance
    /// feel restrictive since the closure cannot close over as many types as
    /// before. The reason for this, though, is to ensure that
    /// [`Store<T>`](crate::Store) can implement both the `Send` and `Sync`
    /// traits.
    ///
    /// Fear not, however, because this isn't as restrictive as it seems! Host
    /// functions are provided a [`Caller<'_, T>`](crate::Caller) argument which
    /// allows access to the host-defined data within the
    /// [`Store`](crate::Store). The `T` type is not required to be any of
    /// `Send`, `Sync`, or `'static`! This means that you can store whatever
    /// you'd like in `T` and have it accessible by all host functions.
    /// Additionally mutable access to `T` is allowed through
    /// [`Caller::data_mut`].
    ///
    /// Most host-defined [`Func`] values provide closures that end up not
    /// actually closing over any values. These zero-sized types will use the
    /// context from [`Caller`] for host-defined information.
    ///
    /// # Errors
    ///
    /// The closure provided here to `wrap` can optionally return a
    /// [`Result<T>`](anyhow::Result). Returning `Ok(t)` represents the host
    /// function successfully completing with the `t` result. Returning
    /// `Err(e)`, however, is equivalent to raising a custom wasm trap.
    /// Execution of WebAssembly does not resume and the stack is unwound to the
    /// original caller of the function where the error is returned.
    ///
    /// For more information about errors in Wasmtime see the [`Trap`]
    /// documentation.
    ///
    /// [`Trap`]: crate::Trap
    ///
    /// # Examples
    ///
    /// First up we can see how simple wasm imports can be implemented, such
    /// as a function that adds its two arguments and returns the result.
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let mut store = Store::<()>::default();
    /// let add = Func::wrap(&mut store, |a: i32, b: i32| a + b);
    /// let module = Module::new(
    ///     store.engine(),
    ///     r#"
    ///         (module
    ///             (import "" "" (func $add (param i32 i32) (result i32)))
    ///             (func (export "foo") (param i32 i32) (result i32)
    ///                 local.get 0
    ///                 local.get 1
    ///                 call $add))
    ///     "#,
    /// )?;
    /// let instance = Instance::new(&mut store, &module, &[add.into()])?;
    /// let foo = instance.get_typed_func::<(i32, i32), i32>(&mut store, "foo")?;
    /// assert_eq!(foo.call(&mut store, (1, 2))?, 3);
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
    /// # let mut store = Store::<()>::default();
    /// let add = Func::wrap(&mut store, |a: i32, b: i32| {
    ///     match a.checked_add(b) {
    ///         Some(i) => Ok(i),
    ///         None => anyhow::bail!("overflow"),
    ///     }
    /// });
    /// let module = Module::new(
    ///     store.engine(),
    ///     r#"
    ///         (module
    ///             (import "" "" (func $add (param i32 i32) (result i32)))
    ///             (func (export "foo") (param i32 i32) (result i32)
    ///                 local.get 0
    ///                 local.get 1
    ///                 call $add))
    ///     "#,
    /// )?;
    /// let instance = Instance::new(&mut store, &module, &[add.into()])?;
    /// let foo = instance.get_typed_func::<(i32, i32), i32>(&mut store, "foo")?;
    /// assert_eq!(foo.call(&mut store, (1, 2))?, 3);
    /// assert!(foo.call(&mut store, (i32::max_value(), 1)).is_err());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// And don't forget all the wasm types are supported!
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let mut store = Store::<()>::default();
    /// let debug = Func::wrap(&mut store, |a: i32, b: u32, c: f32, d: i64, e: u64, f: f64| {
    ///
    ///     println!("a={}", a);
    ///     println!("b={}", b);
    ///     println!("c={}", c);
    ///     println!("d={}", d);
    ///     println!("e={}", e);
    ///     println!("f={}", f);
    /// });
    /// let module = Module::new(
    ///     store.engine(),
    ///     r#"
    ///         (module
    ///             (import "" "" (func $debug (param i32 i32 f32 i64 i64 f64)))
    ///             (func (export "foo")
    ///                 i32.const -1
    ///                 i32.const 1
    ///                 f32.const 2
    ///                 i64.const -3
    ///                 i64.const 3
    ///                 f64.const 4
    ///                 call $debug))
    ///     "#,
    /// )?;
    /// let instance = Instance::new(&mut store, &module, &[debug.into()])?;
    /// let foo = instance.get_typed_func::<(), ()>(&mut store, "foo")?;
    /// foo.call(&mut store, ())?;
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
    /// # let mut store = Store::default();
    /// let log_str = Func::wrap(&mut store, |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
    ///     let mem = match caller.get_export("memory") {
    ///         Some(Extern::Memory(mem)) => mem,
    ///         _ => anyhow::bail!("failed to find host memory"),
    ///     };
    ///     let data = mem.data(&caller)
    ///         .get(ptr as u32 as usize..)
    ///         .and_then(|arr| arr.get(..len as u32 as usize));
    ///     let string = match data {
    ///         Some(data) => match str::from_utf8(data) {
    ///             Ok(s) => s,
    ///             Err(_) => anyhow::bail!("invalid utf-8"),
    ///         },
    ///         None => anyhow::bail!("pointer/length out of bounds"),
    ///     };
    ///     assert_eq!(string, "Hello, world!");
    ///     println!("{}", string);
    ///     Ok(())
    /// });
    /// let module = Module::new(
    ///     store.engine(),
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
    /// let instance = Instance::new(&mut store, &module, &[log_str.into()])?;
    /// let foo = instance.get_typed_func::<(), ()>(&mut store, "foo")?;
    /// foo.call(&mut store, ())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn wrap<T, Params, Results>(
        mut store: impl AsContextMut<Data = T>,
        func: impl IntoFunc<T, Params, Results>,
    ) -> Func {
        let store = store.as_context_mut().0;
        // part of this unsafety is about matching the `T` to a `Store<T>`,
        // which is done through the `AsContextMut` bound above.
        unsafe {
            let host = HostFunc::wrap(store.engine(), func);
            host.into_func(store)
        }
    }

    for_each_function_signature!(generate_wrap_async_func);

    /// Returns the underlying wasm type that this `Func` has.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this function.
    pub fn ty(&self, store: impl AsContext) -> FuncType {
        self.load_ty(&store.as_context().0)
    }

    /// Forcibly loads the type of this function from the `Engine`.
    ///
    /// Note that this is a somewhat expensive method since it requires taking a
    /// lock as well as cloning a type.
    pub(crate) fn load_ty(&self, store: &StoreOpaque) -> FuncType {
        assert!(self.comes_from_same_store(store));
        FuncType::from_shared_type_index(store.engine(), self.type_index(store.store_data()))
    }

    /// Does this function match the given type?
    ///
    /// That is, is this function's type a subtype of the given type?
    pub fn matches_ty(&self, store: impl AsContext, func_ty: &FuncType) -> bool {
        self._matches_ty(store.as_context().0, func_ty)
    }

    pub(crate) fn _matches_ty(&self, store: &StoreOpaque, func_ty: &FuncType) -> bool {
        let actual_ty = self.load_ty(store);
        actual_ty.matches(func_ty)
    }

    pub(crate) fn ensure_matches_ty(&self, store: &StoreOpaque, func_ty: &FuncType) -> Result<()> {
        if !self.comes_from_same_store(store) {
            bail!("function used with wrong store");
        }
        if self._matches_ty(store, func_ty) {
            Ok(())
        } else {
            let actual_ty = self.load_ty(store);
            bail!("type mismatch: expected {func_ty}, found {actual_ty}")
        }
    }

    /// Gets a reference to the `FuncType` for this function.
    ///
    /// Note that this returns both a reference to the type of this function as
    /// well as a reference back to the store itself. This enables using the
    /// `StoreOpaque` while the `FuncType` is also being used (from the
    /// perspective of the borrow-checker) because otherwise the signature would
    /// consider `StoreOpaque` borrowed mutable while `FuncType` is in use.
    fn ty_ref<'a>(&self, store: &'a mut StoreOpaque) -> (&'a FuncType, &'a StoreOpaque) {
        // If we haven't loaded our type into the store yet then do so lazily at
        // this time.
        if store.store_data()[self.0].ty.is_none() {
            let ty = self.load_ty(store);
            store.store_data_mut()[self.0].ty = Some(Box::new(ty));
        }

        (store.store_data()[self.0].ty.as_ref().unwrap(), store)
    }

    pub(crate) fn type_index(&self, data: &StoreData) -> VMSharedTypeIndex {
        data[self.0].sig_index()
    }

    /// Invokes this function with the `params` given and writes returned values
    /// to `results`.
    ///
    /// The `params` here must match the type signature of this `Func`, or an
    /// error will occur. Additionally `results` must have the same
    /// length as the number of results for this function. Calling this function
    /// will synchronously execute the WebAssembly function referenced to get
    /// the results.
    ///
    /// This function will return `Ok(())` if execution completed without a trap
    /// or error of any kind. In this situation the results will be written to
    /// the provided `results` array.
    ///
    /// # Errors
    ///
    /// Any error which occurs throughout the execution of the function will be
    /// returned as `Err(e)`. The [`Error`](anyhow::Error) type can be inspected
    /// for the precise error cause such as:
    ///
    /// * [`Trap`] - indicates that a wasm trap happened and execution was
    ///   halted.
    /// * [`WasmBacktrace`] - optionally included on errors for backtrace
    ///   information of the trap/error.
    /// * Other string-based errors to indicate issues such as type errors with
    ///   `params`.
    /// * Any host-originating error originally returned from a function defined
    ///   via [`Func::new`], for example.
    ///
    /// Errors typically indicate that execution of WebAssembly was halted
    /// mid-way and did not complete after the error condition happened.
    ///
    /// [`Trap`]: crate::Trap
    ///
    /// # Panics
    ///
    /// This function will panic if called on a function belonging to an async
    /// store. Asynchronous stores must always use `call_async`.
    /// initiates a panic. Also panics if `store` does not own this function.
    ///
    /// [`WasmBacktrace`]: crate::WasmBacktrace
    pub fn call(
        &self,
        mut store: impl AsContextMut,
        params: &[Val],
        results: &mut [Val],
    ) -> Result<()> {
        assert!(
            !store.as_context().async_support(),
            "must use `call_async` when async support is enabled on the config",
        );
        let mut store = store.as_context_mut();
        let need_gc = self.call_impl_check_args(&mut store, params, results)?;
        if need_gc {
            store.0.gc();
        }
        unsafe { self.call_impl_do_call(&mut store, params, results) }
    }

    /// Invokes this function in an "unchecked" fashion, reading parameters and
    /// writing results to `params_and_returns`.
    ///
    /// This function is the same as [`Func::call`] except that the arguments
    /// and results both use a different representation. If possible it's
    /// recommended to use [`Func::call`] if safety isn't necessary or to use
    /// [`Func::typed`] in conjunction with [`TypedFunc::call`] since that's
    /// both safer and faster than this method of invoking a function.
    ///
    /// Note that if this function takes `externref` arguments then it will
    /// **not** automatically GC unlike the [`Func::call`] and
    /// [`TypedFunc::call`] functions. This means that if this function is
    /// invoked many times with new `ExternRef` values and no other GC happens
    /// via any other means then no values will get collected.
    ///
    /// # Errors
    ///
    /// For more information about errors see the [`Func::call`] documentation.
    ///
    /// # Unsafety
    ///
    /// This function is unsafe because the `params_and_returns` argument is not
    /// validated at all. It must uphold invariants such as:
    ///
    /// * It's a valid pointer to an array
    /// * It has enough space to store all parameters
    /// * It has enough space to store all results (not at the same time as
    ///   parameters)
    /// * Parameters are initially written to the array and have the correct
    ///   types and such.
    /// * Reference types like `externref` and `funcref` are valid at the
    ///   time of this call and for the `store` specified.
    ///
    /// These invariants are all upheld for you with [`Func::call`] and
    /// [`TypedFunc::call`].
    pub unsafe fn call_unchecked(
        &self,
        mut store: impl AsContextMut,
        params_and_returns: *mut ValRaw,
        params_and_returns_capacity: usize,
    ) -> Result<()> {
        let mut store = store.as_context_mut();
        let data = &store.0.store_data()[self.0];
        let func_ref = data.export().func_ref;
        Self::call_unchecked_raw(
            &mut store,
            func_ref,
            params_and_returns,
            params_and_returns_capacity,
        )
    }

    pub(crate) unsafe fn call_unchecked_raw<T>(
        store: &mut StoreContextMut<'_, T>,
        func_ref: NonNull<VMFuncRef>,
        params_and_returns: *mut ValRaw,
        params_and_returns_capacity: usize,
    ) -> Result<()> {
        invoke_wasm_and_catch_traps(store, |caller| {
            let func_ref = func_ref.as_ref();
            (func_ref.array_call)(
                func_ref.vmctx,
                caller.cast::<VMOpaqueContext>(),
                params_and_returns,
                params_and_returns_capacity,
            )
        })
    }

    /// Converts the raw representation of a `funcref` into an `Option<Func>`
    ///
    /// This is intended to be used in conjunction with [`Func::new_unchecked`],
    /// [`Func::call_unchecked`], and [`ValRaw`] with its `funcref` field.
    ///
    /// # Unsafety
    ///
    /// This function is not safe because `raw` is not validated at all. The
    /// caller must guarantee that `raw` is owned by the `store` provided and is
    /// valid within the `store`.
    pub unsafe fn from_raw(mut store: impl AsContextMut, raw: *mut c_void) -> Option<Func> {
        Self::_from_raw(store.as_context_mut().0, raw)
    }

    pub(crate) unsafe fn _from_raw(store: &mut StoreOpaque, raw: *mut c_void) -> Option<Func> {
        Func::from_vm_func_ref(store, raw.cast())
    }

    /// Extracts the raw value of this `Func`, which is owned by `store`.
    ///
    /// This function returns a value that's suitable for writing into the
    /// `funcref` field of the [`ValRaw`] structure.
    ///
    /// # Unsafety
    ///
    /// The returned value is only valid for as long as the store is alive and
    /// this function is properly rooted within it. Additionally this function
    /// should not be liberally used since it's a very low-level knob.
    pub unsafe fn to_raw(&self, mut store: impl AsContextMut) -> *mut c_void {
        self.vm_func_ref(store.as_context_mut().0).as_ptr().cast()
    }

    /// Invokes this function with the `params` given, returning the results
    /// asynchronously.
    ///
    /// This function is the same as [`Func::call`] except that it is
    /// asynchronous. This is only compatible with stores associated with an
    /// [asynchronous config](crate::Config::async_support).
    ///
    /// It's important to note that the execution of WebAssembly will happen
    /// synchronously in the `poll` method of the future returned from this
    /// function. Wasmtime does not manage its own thread pool or similar to
    /// execute WebAssembly in. Future `poll` methods are generally expected to
    /// resolve quickly, so it's recommended that you run or poll this future
    /// in a "blocking context".
    ///
    /// For more information see the documentation on [asynchronous
    /// configs](crate::Config::async_support).
    ///
    /// # Errors
    ///
    /// For more information on errors see the [`Func::call`] documentation.
    ///
    /// # Panics
    ///
    /// Panics if this is called on a function in a synchronous store. This
    /// only works with functions defined within an asynchronous store. Also
    /// panics if `store` does not own this function.
    #[cfg(feature = "async")]
    pub async fn call_async<T>(
        &self,
        mut store: impl AsContextMut<Data = T>,
        params: &[Val],
        results: &mut [Val],
    ) -> Result<()>
    where
        T: Send,
    {
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "cannot use `call_async` without enabling async support in the config",
        );
        let need_gc = self.call_impl_check_args(&mut store, params, results)?;
        if need_gc {
            store.0.gc_async().await;
        }
        let result = store
            .on_fiber(|store| unsafe { self.call_impl_do_call(store, params, results) })
            .await??;
        Ok(result)
    }

    /// Perform dynamic checks that the arguments given to us match
    /// the signature of this function and are appropriate to pass to this
    /// function.
    ///
    /// This involves checking to make sure we have the right number and types
    /// of arguments as well as making sure everything is from the same `Store`.
    ///
    /// This must be called just before `call_impl_do_call`.
    ///
    /// Returns whether we need to GC before calling `call_impl_do_call`.
    fn call_impl_check_args<T>(
        &self,
        store: &mut StoreContextMut<'_, T>,
        params: &[Val],
        results: &mut [Val],
    ) -> Result<bool> {
        let (ty, opaque) = self.ty_ref(store.0);
        if ty.params().len() != params.len() {
            bail!(
                "expected {} arguments, got {}",
                ty.params().len(),
                params.len()
            );
        }
        if ty.results().len() != results.len() {
            bail!(
                "expected {} results, got {}",
                ty.results().len(),
                results.len()
            );
        }
        for (ty, arg) in ty.params().zip(params) {
            arg.ensure_matches_ty(opaque, &ty)
                .context("argument type mismatch")?;
            if !arg.comes_from_same_store(opaque) {
                bail!("cross-`Store` values are not currently supported");
            }
        }

        #[cfg(feature = "gc")]
        {
            // Check whether we need to GC before calling into Wasm.
            //
            // For example, with the DRC collector, whenever we pass GC refs
            // from host code to Wasm code, they go into the
            // `VMGcRefActivationsTable`. But the table might be at capacity
            // already. If it is at capacity (unlikely) then we need to do a GC
            // to free up space.
            let num_gc_refs = ty.as_wasm_func_type().non_i31_gc_ref_params_count();
            if let Some(num_gc_refs) = NonZeroUsize::new(num_gc_refs) {
                return Ok(opaque
                    .gc_store()?
                    .gc_heap
                    .need_gc_before_entering_wasm(num_gc_refs));
            }
        }

        Ok(false)
    }

    /// Do the actual call into Wasm.
    ///
    /// # Safety
    ///
    /// You must have type checked the arguments by calling
    /// `call_impl_check_args` immediately before calling this function. It is
    /// only safe to call this function if that one did not return an error.
    unsafe fn call_impl_do_call<T>(
        &self,
        store: &mut StoreContextMut<'_, T>,
        params: &[Val],
        results: &mut [Val],
    ) -> Result<()> {
        // Store the argument values into `values_vec`.
        let (ty, _) = self.ty_ref(store.0);
        let values_vec_size = params.len().max(ty.results().len());
        let mut values_vec = store.0.take_wasm_val_raw_storage();
        debug_assert!(values_vec.is_empty());
        values_vec.resize_with(values_vec_size, || ValRaw::v128(0));
        for (arg, slot) in params.iter().cloned().zip(&mut values_vec) {
            unsafe {
                *slot = arg.to_raw(&mut *store)?;
            }
        }

        unsafe {
            self.call_unchecked(&mut *store, values_vec.as_mut_ptr(), values_vec_size)?;
        }

        for ((i, slot), val) in results.iter_mut().enumerate().zip(&values_vec) {
            let ty = self.ty_ref(store.0).0.results().nth(i).unwrap();
            *slot = unsafe { Val::from_raw(&mut *store, *val, ty) };
        }
        values_vec.truncate(0);
        store.0.save_wasm_val_raw_storage(values_vec);
        Ok(())
    }

    #[inline]
    pub(crate) fn vm_func_ref(&self, store: &mut StoreOpaque) -> NonNull<VMFuncRef> {
        let func_data = &mut store.store_data_mut()[self.0];
        let func_ref = func_data.export().func_ref;
        if unsafe { func_ref.as_ref().wasm_call.is_some() } {
            return func_ref;
        }

        if let Some(in_store) = func_data.in_store_func_ref {
            in_store.as_non_null()
        } else {
            unsafe {
                // Move this uncommon/slow path out of line.
                self.copy_func_ref_into_store_and_fill(store, func_ref)
            }
        }
    }

    unsafe fn copy_func_ref_into_store_and_fill(
        &self,
        store: &mut StoreOpaque,
        func_ref: NonNull<VMFuncRef>,
    ) -> NonNull<VMFuncRef> {
        let func_ref = store.func_refs().push(func_ref.as_ref().clone());
        store.store_data_mut()[self.0].in_store_func_ref = Some(SendSyncPtr::new(func_ref));
        store.fill_func_refs();
        func_ref
    }

    pub(crate) unsafe fn from_wasmtime_function(
        export: ExportFunction,
        store: &mut StoreOpaque,
    ) -> Self {
        Func::from_func_kind(FuncKind::StoreOwned { export }, store)
    }

    fn from_func_kind(kind: FuncKind, store: &mut StoreOpaque) -> Self {
        Func(store.store_data_mut().insert(FuncData {
            kind,
            in_store_func_ref: None,
            ty: None,
        }))
    }

    pub(crate) fn vmimport(&self, store: &mut StoreOpaque, module: &Module) -> VMFunctionImport {
        unsafe {
            let f = {
                let func_data = &mut store.store_data_mut()[self.0];
                // If we already patched this `funcref.wasm_call` and saved a
                // copy in the store, use the patched version. Otherwise, use
                // the potentially un-patched version.
                if let Some(func_ref) = func_data.in_store_func_ref {
                    func_ref.as_non_null()
                } else {
                    func_data.export().func_ref
                }
            };
            VMFunctionImport {
                wasm_call: if let Some(wasm_call) = f.as_ref().wasm_call {
                    wasm_call
                } else {
                    // Assert that this is a native-call function, since those
                    // are the only ones that could be missing a `wasm_call`
                    // trampoline.
                    let _ = VMNativeCallHostFuncContext::from_opaque(f.as_ref().vmctx);

                    let sig = self.type_index(store.store_data());
                    module.runtime_info().wasm_to_native_trampoline(sig).expect(
                        "if the wasm is importing a function of a given type, it must have the \
                         type's trampoline",
                    )
                },
                native_call: f.as_ref().native_call,
                array_call: f.as_ref().array_call,
                vmctx: f.as_ref().vmctx,
            }
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        store.store_data().contains(self.0)
    }

    fn invoke_host_func_for_wasm<T>(
        mut caller: Caller<'_, T>,
        ty: &FuncType,
        values_vec: &mut [ValRaw],
        func: &dyn Fn(Caller<'_, T>, &[Val], &mut [Val]) -> Result<()>,
    ) -> Result<()> {
        // Translate the raw JIT arguments in `values_vec` into a `Val` which
        // we'll be passing as a slice. The storage for our slice-of-`Val` we'll
        // be taking from the `Store`. We preserve our slice back into the
        // `Store` after the hostcall, ideally amortizing the cost of allocating
        // the storage across wasm->host calls.
        //
        // Note that we have a dynamic guarantee that `values_vec` is the
        // appropriate length to both read all arguments from as well as store
        // all results into.
        let mut val_vec = caller.store.0.take_hostcall_val_storage();
        debug_assert!(val_vec.is_empty());
        let nparams = ty.params().len();
        val_vec.reserve(nparams + ty.results().len());
        for (i, ty) in ty.params().enumerate() {
            val_vec.push(unsafe { Val::from_raw(&mut caller.store, values_vec[i], ty) })
        }

        val_vec.extend((0..ty.results().len()).map(|_| Val::null_func_ref()));
        let (params, results) = val_vec.split_at_mut(nparams);
        func(caller.sub_caller(), params, results)?;

        // Unlike our arguments we need to dynamically check that the return
        // values produced are correct. There could be a bug in `func` that
        // produces the wrong number, wrong types, or wrong stores of
        // values, and we need to catch that here.
        for (i, (ret, ty)) in results.iter().zip(ty.results()).enumerate() {
            ret.ensure_matches_ty(caller.store.0, &ty)
                .context("function attempted to return an incompatible value")?;
            unsafe {
                values_vec[i] = ret.to_raw(&mut caller.store)?;
            }
        }

        // Restore our `val_vec` back into the store so it's usable for the next
        // hostcall to reuse our own storage.
        val_vec.truncate(0);
        caller.store.0.save_hostcall_val_storage(val_vec);
        Ok(())
    }

    /// Attempts to extract a typed object from this `Func` through which the
    /// function can be called.
    ///
    /// This function serves as an alternative to [`Func::call`] and
    /// [`Func::call_async`]. This method performs a static type check (using
    /// the `Params` and `Results` type parameters on the underlying wasm
    /// function. If the type check passes then a `TypedFunc` object is returned,
    /// otherwise an error is returned describing the typecheck failure.
    ///
    /// The purpose of this relative to [`Func::call`] is that it's much more
    /// efficient when used to invoke WebAssembly functions. With the types
    /// statically known far less setup/teardown is required when invoking
    /// WebAssembly. If speed is desired then this function is recommended to be
    /// used instead of [`Func::call`] (which is more general, hence its
    /// slowdown).
    ///
    /// The `Params` type parameter is used to describe the parameters of the
    /// WebAssembly function. This can either be a single type (like `i32`), or
    /// a tuple of types representing the list of parameters (like `(i32, f32,
    /// f64)`). Additionally you can use `()` to represent that the function has
    /// no parameters.
    ///
    /// The `Results` type parameter is used to describe the results of the
    /// function. This behaves the same way as `Params`, but just for the
    /// results of the function.
    ///
    /// # Translating Between WebAssembly and Rust Types
    ///
    /// Translation between Rust types and WebAssembly types looks like:
    ///
    /// | WebAssembly                               | Rust                                  |
    /// |-------------------------------------------|---------------------------------------|
    /// | `i32`                                     | `i32` or `u32`                        |
    /// | `i64`                                     | `i64` or `u64`                        |
    /// | `f32`                                     | `f32`                                 |
    /// | `f64`                                     | `f64`                                 |
    /// | `externref` aka `(ref null extern)`       | `Option<ExternRef>`                   |
    /// | `(ref extern)`                            | `ExternRef`                           |
    /// | `(ref noextern)`                          | `NoExtern`                            |
    /// | `nullexternref` aka `(ref null noextern)` | `Option<NoExtern>`                    |
    /// | `anyref` aka `(ref null any)`             | `Option<AnyRef>`                      |
    /// | `(ref any)`                               | `AnyRef`                              |
    /// | `i31ref` aka `(ref null i31)`             | `Option<I31>`                         |
    /// | `(ref i31)`                               | `I31`                                 |
    /// | `funcref` aka `(ref null func)`           | `Option<Func>`                        |
    /// | `(ref func)`                              | `Func`                                |
    /// | `(ref null <func type index>)`            | `Option<Func>`                        |
    /// | `(ref <func type index>)`                 | `Func`                                |
    /// | `nullfuncref` aka `(ref null nofunc)`     | `Option<NoFunc>`                      |
    /// | `(ref nofunc)`                            | `NoFunc`                              |
    /// | `v128`                                    | `V128` on `x86-64` and `aarch64` only |
    ///
    /// (Note that this mapping is the same as that of [`Func::wrap`]).
    ///
    /// Note that once the [`TypedFunc`] return value is acquired you'll use either
    /// [`TypedFunc::call`] or [`TypedFunc::call_async`] as necessary to actually invoke
    /// the function. This method does not invoke any WebAssembly code, it
    /// simply performs a typecheck before returning the [`TypedFunc`] value.
    ///
    /// This method also has a convenience wrapper as
    /// [`Instance::get_typed_func`](crate::Instance::get_typed_func) to
    /// directly get a typed function value from an
    /// [`Instance`](crate::Instance).
    ///
    /// ## Subtyping
    ///
    /// For result types, you can always use a supertype of the WebAssembly
    /// function's actual declared result type. For example, if the WebAssembly
    /// function was declared with type `(func (result nullfuncref))` you could
    /// successfully call `f.typed::<(), Option<Func>>()` because `Option<Func>`
    /// corresponds to `funcref`, which is a supertype of `nullfuncref`.
    ///
    /// For parameter types, you can always use a subtype of the WebAssembly
    /// function's actual declared parameter type. For example, if the
    /// WebAssembly function was declared with type `(func (param (ref null
    /// func)))` you could successfully call `f.typed::<Func, ()>()` because
    /// `Func` corresponds to `(ref func)`, which is a subtype of `(ref null
    /// func)`.
    ///
    /// Additionally, for functions which take a reference to a concrete type as
    /// a parameter, you can also use the concrete type's supertype. Consider a
    /// WebAssembly function that takes a reference to a function with a
    /// concrete type: `(ref null <func type index>)`. In this scenario, there
    /// is no static `wasmtime::Foo` Rust type that corresponds to that
    /// particular Wasm-defined concrete reference type because Wasm modules are
    /// loaded dynamically at runtime. You *could* do `f.typed::<Option<NoFunc>,
    /// ()>()`, and while that is correctly typed and valid, it is often overly
    /// restrictive. The only value you could call the resulting typed function
    /// with is the null function reference, but we'd like to call it with
    /// non-null function references that happen to be of the correct
    /// type. Therefore, `f.typed<Option<Func>, ()>()` is also allowed in this
    /// case, even though `Option<Func>` represents `(ref null func)` which is
    /// the supertype, not subtype, of `(ref null <func type index>)`. This does
    /// imply some minimal dynamic type checks in this case, but it is supported
    /// for better ergonomics, to enable passing non-null references into the
    /// function.
    ///
    /// # Errors
    ///
    /// This function will return an error if `Params` or `Results` does not
    /// match the native type of this WebAssembly function.
    ///
    /// # Panics
    ///
    /// This method will panic if `store` does not own this function.
    ///
    /// # Examples
    ///
    /// An end-to-end example of calling a function which takes no parameters
    /// and has no results:
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// let engine = Engine::default();
    /// let mut store = Store::new(&engine, ());
    /// let module = Module::new(&engine, r#"(module (func (export "foo")))"#)?;
    /// let instance = Instance::new(&mut store, &module, &[])?;
    /// let foo = instance.get_func(&mut store, "foo").expect("export wasn't a function");
    ///
    /// // Note that this call can fail due to the typecheck not passing, but
    /// // in our case we statically know the module so we know this should
    /// // pass.
    /// let typed = foo.typed::<(), ()>(&store)?;
    ///
    /// // Note that this can fail if the wasm traps at runtime.
    /// typed.call(&mut store, ())?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// You can also pass in multiple parameters and get a result back
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn foo(add: &Func, mut store: Store<()>) -> anyhow::Result<()> {
    /// let typed = add.typed::<(i32, i64), f32>(&store)?;
    /// assert_eq!(typed.call(&mut store, (1, 2))?, 3.0);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// and similarly if a function has multiple results you can bind that too
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn foo(add_with_overflow: &Func, mut store: Store<()>) -> anyhow::Result<()> {
    /// let typed = add_with_overflow.typed::<(u32, u32), (u32, i32)>(&store)?;
    /// let (result, overflow) = typed.call(&mut store, (u32::max_value(), 2))?;
    /// assert_eq!(result, 1);
    /// assert_eq!(overflow, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn typed<Params, Results>(
        &self,
        store: impl AsContext,
    ) -> Result<TypedFunc<Params, Results>>
    where
        Params: WasmParams,
        Results: WasmResults,
    {
        // Type-check that the params/results are all valid
        let store = store.as_context().0;
        let ty = self.load_ty(store);
        Params::typecheck(store.engine(), ty.params(), TypeCheckPosition::Param)
            .context("type mismatch with parameters")?;
        Results::typecheck(store.engine(), ty.results(), TypeCheckPosition::Result)
            .context("type mismatch with results")?;

        // and then we can construct the typed version of this function
        // (unsafely), which should be safe since we just did the type check above.
        unsafe { Ok(TypedFunc::_new_unchecked(store, *self)) }
    }

    /// Get a stable hash key for this function.
    ///
    /// Even if the same underlying function is added to the `StoreData`
    /// multiple times and becomes multiple `wasmtime::Func`s, this hash key
    /// will be consistent across all of these functions.
    #[allow(dead_code)] // Not used yet, but added for consistency.
    pub(crate) fn hash_key(&self, store: &mut StoreOpaque) -> impl core::hash::Hash + Eq {
        self.vm_func_ref(store).as_ptr() as usize
    }
}

/// Prepares for entrance into WebAssembly.
///
/// This function will set up context such that `closure` is allowed to call a
/// raw trampoline or a raw WebAssembly function. This *must* be called to do
/// things like catch traps and set up GC properly.
///
/// The `closure` provided receives a default "caller" `VMContext` parameter it
/// can pass to the called wasm function, if desired.
pub(crate) fn invoke_wasm_and_catch_traps<T>(
    store: &mut StoreContextMut<'_, T>,
    closure: impl FnMut(*mut VMContext),
) -> Result<()> {
    unsafe {
        let exit = enter_wasm(store);

        if let Err(trap) = store.0.call_hook(CallHook::CallingWasm) {
            exit_wasm(store, exit);
            return Err(trap);
        }
        let result = crate::runtime::vm::catch_traps(
            store.0.signal_handler(),
            store.0.engine().config().wasm_backtrace,
            store.0.engine().config().coredump_on_trap,
            store.0.default_caller(),
            closure,
        );
        exit_wasm(store, exit);
        store.0.call_hook(CallHook::ReturningFromWasm)?;
        result.map_err(|t| crate::trap::from_runtime_box(store.0, t))
    }
}

/// This function is called to register state within `Store` whenever
/// WebAssembly is entered within the `Store`.
///
/// This function sets up various limits such as:
///
/// * The stack limit. This is what ensures that we limit the stack space
///   allocated by WebAssembly code and it's relative to the initial stack
///   pointer that called into wasm.
///
/// This function may fail if the stack limit can't be set because an
/// interrupt already happened.
fn enter_wasm<T>(store: &mut StoreContextMut<'_, T>) -> Option<usize> {
    // If this is a recursive call, e.g. our stack limit is already set, then
    // we may be able to skip this function.
    //
    // For synchronous stores there's nothing else to do because all wasm calls
    // happen synchronously and on the same stack. This means that the previous
    // stack limit will suffice for the next recursive call.
    //
    // For asynchronous stores then each call happens on a separate native
    // stack. This means that the previous stack limit is no longer relevant
    // because we're on a separate stack.
    if unsafe { *store.0.runtime_limits().stack_limit.get() } != usize::MAX
        && !store.0.async_support()
    {
        return None;
    }

    // Ignore this stack pointer business on miri since we can't execute wasm
    // anyway and the concept of a stack pointer on miri is a bit nebulous
    // regardless.
    if cfg!(miri) {
        return None;
    }

    let stack_pointer = crate::runtime::vm::get_stack_pointer();

    // Determine the stack pointer where, after which, any wasm code will
    // immediately trap. This is checked on the entry to all wasm functions.
    //
    // Note that this isn't 100% precise. We are requested to give wasm
    // `max_wasm_stack` bytes, but what we're actually doing is giving wasm
    // probably a little less than `max_wasm_stack` because we're
    // calculating the limit relative to this function's approximate stack
    // pointer. Wasm will be executed on a frame beneath this one (or next
    // to it). In any case it's expected to be at most a few hundred bytes
    // of slop one way or another. When wasm is typically given a MB or so
    // (a million bytes) the slop shouldn't matter too much.
    //
    // After we've got the stack limit then we store it into the `stack_limit`
    // variable.
    let wasm_stack_limit = stack_pointer - store.engine().config().max_wasm_stack;
    let prev_stack = unsafe {
        mem::replace(
            &mut *store.0.runtime_limits().stack_limit.get(),
            wasm_stack_limit,
        )
    };

    Some(prev_stack)
}

fn exit_wasm<T>(store: &mut StoreContextMut<'_, T>, prev_stack: Option<usize>) {
    // If we don't have a previous stack pointer to restore, then there's no
    // cleanup we need to perform here.
    let prev_stack = match prev_stack {
        Some(stack) => stack,
        None => return,
    };

    unsafe {
        *store.0.runtime_limits().stack_limit.get() = prev_stack;
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
    // Same as `WasmTy::Abi`.
    #[doc(hidden)]
    type Abi: 'static + Copy;
    #[doc(hidden)]
    type Retptr: Copy;

    // Same as `WasmTy::compatible_with_store`.
    #[doc(hidden)]
    fn compatible_with_store(&self, store: &StoreOpaque) -> bool;

    // Similar to `WasmTy::into_abi_for_arg` but used when host code is
    // returning a value into Wasm, rather than host code passing an argument to
    // a Wasm call. Unlike `into_abi_for_arg`, implementors of this method can
    // raise traps, which means that callers must ensure that
    // `invoke_wasm_and_catch_traps` is on the stack, and therefore this method
    // is unsafe.
    #[doc(hidden)]
    unsafe fn into_abi_for_ret(
        self,
        store: &mut AutoAssertNoGc<'_>,
        ptr: Self::Retptr,
    ) -> Result<Self::Abi>;

    #[doc(hidden)]
    fn func_type(engine: &Engine, params: impl Iterator<Item = ValType>) -> FuncType;

    #[doc(hidden)]
    unsafe fn wrap_trampoline(ptr: *mut ValRaw, f: impl FnOnce(Self::Retptr) -> Self::Abi);

    // Utilities used to convert an instance of this type to a `Result`
    // explicitly, used when wrapping async functions which always bottom-out
    // in a function that returns a trap because futures can be cancelled.
    #[doc(hidden)]
    type Fallible: WasmRet<Abi = Self::Abi, Retptr = Self::Retptr>;
    #[doc(hidden)]
    fn into_fallible(self) -> Self::Fallible;
    #[doc(hidden)]
    fn fallible_from_error(error: Error) -> Self::Fallible;
}

unsafe impl<T> WasmRet for T
where
    T: WasmTy,
{
    type Abi = <T as WasmTy>::Abi;
    type Retptr = ();
    type Fallible = Result<T>;

    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        <Self as WasmTy>::compatible_with_store(self, store)
    }

    unsafe fn into_abi_for_ret(
        self,
        store: &mut AutoAssertNoGc<'_>,
        _retptr: (),
    ) -> Result<Self::Abi> {
        <Self as WasmTy>::into_abi(self, store)
    }

    fn func_type(engine: &Engine, params: impl Iterator<Item = ValType>) -> FuncType {
        FuncType::new(engine, params, Some(<Self as WasmTy>::valtype()))
    }

    unsafe fn wrap_trampoline(ptr: *mut ValRaw, f: impl FnOnce(Self::Retptr) -> Self::Abi) {
        T::abi_into_raw(f(()), ptr);
    }

    fn into_fallible(self) -> Result<T> {
        Ok(self)
    }

    fn fallible_from_error(error: Error) -> Result<T> {
        Err(error)
    }
}

unsafe impl<T> WasmRet for Result<T>
where
    T: WasmRet,
{
    type Abi = <T as WasmRet>::Abi;
    type Retptr = <T as WasmRet>::Retptr;
    type Fallible = Self;

    fn compatible_with_store(&self, store: &StoreOpaque) -> bool {
        match self {
            Ok(x) => <T as WasmRet>::compatible_with_store(x, store),
            Err(_) => true,
        }
    }

    unsafe fn into_abi_for_ret(
        self,
        store: &mut AutoAssertNoGc<'_>,
        retptr: Self::Retptr,
    ) -> Result<Self::Abi> {
        self.and_then(|val| val.into_abi_for_ret(store, retptr))
    }

    fn func_type(engine: &Engine, params: impl Iterator<Item = ValType>) -> FuncType {
        T::func_type(engine, params)
    }

    unsafe fn wrap_trampoline(ptr: *mut ValRaw, f: impl FnOnce(Self::Retptr) -> Self::Abi) {
        T::wrap_trampoline(ptr, f)
    }

    fn into_fallible(self) -> Result<T> {
        self
    }

    fn fallible_from_error(error: Error) -> Result<T> {
        Err(error)
    }
}

macro_rules! impl_wasm_host_results {
    ($n:tt $($t:ident)*) => (
        #[allow(non_snake_case)]
        unsafe impl<$($t),*> WasmRet for ($($t,)*)
        where
            $($t: WasmTy,)*
            ($($t::Abi,)*): HostAbi,
        {
            type Abi = <($($t::Abi,)*) as HostAbi>::Abi;
            type Retptr = <($($t::Abi,)*) as HostAbi>::Retptr;
            type Fallible = Result<Self>;

            #[inline]
            fn compatible_with_store(&self, _store: &StoreOpaque) -> bool {
                let ($($t,)*) = self;
                $( $t.compatible_with_store(_store) && )* true
            }

            #[inline]
            unsafe fn into_abi_for_ret(
                self,
                _store: &mut AutoAssertNoGc<'_>,
                ptr: Self::Retptr,
            ) -> Result<Self::Abi> {
                let ($($t,)*) = self;
                let abi = ($($t.into_abi(_store)?,)*);
                Ok(<($($t::Abi,)*) as HostAbi>::into_abi(abi, ptr))
            }

            fn func_type(engine: &Engine, params: impl Iterator<Item = ValType>) -> FuncType {
                FuncType::new(
                    engine,
                    params,
                    IntoIterator::into_iter([$($t::valtype(),)*]),
                )
            }

            #[allow(unused_assignments)]
            unsafe fn wrap_trampoline(mut _ptr: *mut ValRaw, f: impl FnOnce(Self::Retptr) -> Self::Abi) {
                let ($($t,)*) = <($($t::Abi,)*) as HostAbi>::call(f);
                $(
                    $t::abi_into_raw($t, _ptr);
                    _ptr = _ptr.add(1);
                )*
            }

            #[inline]
            fn into_fallible(self) -> Result<Self> {
                Ok(self)
            }

            #[inline]
            fn fallible_from_error(error: Error) -> Result<Self> {
                Err(error)
            }
        }
    )
}

for_each_function_signature!(impl_wasm_host_results);

// Internal trait representing how to communicate tuples of return values across
// an ABI boundary. This internally corresponds to the "wasmtime" ABI inside of
// cranelift itself. Notably the first element of each tuple is returned via the
// typical system ABI (e.g. systemv or fastcall depending on platform) and all
// other values are returned packed via the stack.
//
// This trait helps to encapsulate all the details of that.
#[doc(hidden)]
pub trait HostAbi {
    // A value returned from native functions which return `Self`
    type Abi: Copy;
    // A return pointer, added to the end of the argument list, for native
    // functions that return `Self`. Note that a 0-sized type here should get
    // elided at the ABI level.
    type Retptr: Copy;

    // Converts a value of `self` into its components. Stores necessary values
    // into `ptr` and then returns whatever needs to be returned from the
    // function.
    unsafe fn into_abi(self, ptr: Self::Retptr) -> Self::Abi;

    // Calls `f` with a suitably sized return area and requires `f` to return
    // the raw abi value of the first element of our tuple. This will then
    // unpack the `Retptr` and assemble it with `Self::Abi` to return an
    // instance of the whole tuple.
    unsafe fn call(f: impl FnOnce(Self::Retptr) -> Self::Abi) -> Self;
}

macro_rules! impl_host_abi {
    // Base case, everything is `()`
    (0) => {
        impl HostAbi for () {
            type Abi = ();
            type Retptr = ();

            #[inline]
            unsafe fn into_abi(self, _ptr: Self::Retptr) -> Self::Abi {}

            #[inline]
            unsafe fn call(f: impl FnOnce(Self::Retptr) -> Self::Abi) -> Self {
                f(())
            }
        }
    };

    // In the 1-case the retptr is not present, so it's a 0-sized value.
    (1 $a:ident) => {
        impl<$a: Copy> HostAbi for ($a,) {
            type Abi = $a;
            type Retptr = ();

            unsafe fn into_abi(self, _ptr: Self::Retptr) -> Self::Abi {
                self.0
            }

            unsafe fn call(f: impl FnOnce(Self::Retptr) -> Self::Abi) -> Self {
                (f(()),)
            }
        }
    };

    // This is where the more interesting case happens. The first element of the
    // tuple is returned via `Abi` and all other elements are returned via
    // `Retptr`. We create a `TupleRetNN` structure to represent all of the
    // return values here.
    //
    // Also note that this isn't implemented for the old backend right now due
    // to the original author not really being sure how to implement this in the
    // old backend.
    ($n:tt $t:ident $($u:ident)*) => {paste::paste!{
        #[doc(hidden)]
        #[allow(non_snake_case)]
        #[repr(C)]
        pub struct [<TupleRet $n>]<$($u,)*> {
            $($u: $u,)*
        }

        #[allow(non_snake_case, unused_assignments)]
        impl<$t: Copy, $($u: Copy,)*> HostAbi for ($t, $($u,)*) {
            type Abi = $t;
            type Retptr = *mut [<TupleRet $n>]<$($u,)*>;

            unsafe fn into_abi(self, ptr: Self::Retptr) -> Self::Abi {
                let ($t, $($u,)*) = self;
                // Store the tail of our tuple into the return pointer...
                $((*ptr).$u = $u;)*
                // ... and return the head raw.
                $t
            }

            unsafe fn call(f: impl FnOnce(Self::Retptr) -> Self::Abi) -> Self {
                // Create space to store all the return values and then invoke
                // the function.
                let mut space = core::mem::MaybeUninit::uninit();
                let t = f(space.as_mut_ptr());
                let space = space.assume_init();

                // Use the return value as the head of the tuple and unpack our
                // return area to get the rest of the tuple.
                (t, $(space.$u,)*)
            }
        }
    }};
}

for_each_function_signature!(impl_host_abi);

/// Internal trait implemented for all arguments that can be passed to
/// [`Func::wrap`] and [`Linker::func_wrap`](crate::Linker::func_wrap).
///
/// This trait should not be implemented by external users, it's only intended
/// as an implementation detail of this crate.
pub trait IntoFunc<T, Params, Results>: Send + Sync + 'static {
    /// Convert this function into a `VM{Array,Native}CallHostFuncContext` and
    /// internal `VMFuncRef`.
    #[doc(hidden)]
    fn into_func(self, engine: &Engine) -> HostContext;
}

/// A structure representing the caller's context when creating a function
/// via [`Func::wrap`].
///
/// This structure can be taken as the first parameter of a closure passed to
/// [`Func::wrap`] or other constructors, and serves two purposes:
///
/// * First consumers can use [`Caller<'_, T>`](crate::Caller) to get access to
///   [`StoreContextMut<'_, T>`](crate::StoreContextMut) and/or get access to
///   `T` itself. This means that the [`Caller`] type can serve as a proxy to
///   the original [`Store`](crate::Store) itself and is used to satisfy
///   [`AsContext`] and [`AsContextMut`] bounds.
///
/// * Second a [`Caller`] can be used as the name implies, learning about the
///   caller's context, namely it's exported memory and exported functions. This
///   allows functions which take pointers as arguments to easily read the
///   memory the pointers point into, or if a function is expected to call
///   malloc in the wasm module to reserve space for the output you can do that.
///
/// Host functions which want access to [`Store`](crate::Store)-level state are
/// recommended to use this type.
pub struct Caller<'a, T> {
    pub(crate) store: StoreContextMut<'a, T>,
    caller: &'a crate::runtime::vm::Instance,
}

impl<T> Caller<'_, T> {
    unsafe fn with<F, R>(caller: *mut VMContext, f: F) -> R
    where
        // The closure must be valid for any `Caller` it is given; it doesn't
        // get to choose the `Caller`'s lifetime.
        F: for<'a> FnOnce(Caller<'a, T>) -> R,
        // And the return value must not borrow from the caller/store.
        R: 'static,
    {
        assert!(!caller.is_null());
        crate::runtime::vm::Instance::from_vmctx(caller, |instance| {
            let store = StoreContextMut::from_raw(instance.store());
            let gc_lifo_scope = store.0.gc_roots().enter_lifo_scope();

            let ret = f(Caller {
                store,
                caller: &instance,
            });

            // Safe to recreate a mutable borrow of the store because `ret`
            // cannot be borrowing from the store.
            let store = StoreContextMut::<T>::from_raw(instance.store());
            store.0.exit_gc_lifo_scope(gc_lifo_scope);

            ret
        })
    }

    fn sub_caller(&mut self) -> Caller<'_, T> {
        Caller {
            store: self.store.as_context_mut(),
            caller: self.caller,
        }
    }

    /// Looks up an export from the caller's module by the `name` given.
    ///
    /// This is a low-level function that's typically used to implement passing
    /// of pointers or indices between core Wasm instances, where the callee
    /// needs to consult the caller's exports to perform memory management and
    /// resolve the references.
    ///
    /// For comparison, in components, the component model handles translating
    /// arguments from one component instance to another and managing memory, so
    /// that callees don't need to be aware of their callers, which promotes
    /// virtualizability of APIs.
    ///
    /// # Return
    ///
    /// If an export with the `name` provided was found, then it is returned as an
    /// `Extern`. There are a number of situations, however, where the export may not
    /// be available:
    ///
    /// * The caller instance may not have an export named `name`
    /// * There may not be a caller available, for example if `Func` was called
    ///   directly from host code.
    ///
    /// It's recommended to take care when calling this API and gracefully
    /// handling a `None` return value.
    pub fn get_export(&mut self, name: &str) -> Option<Extern> {
        // All instances created have a `host_state` with a pointer pointing
        // back to themselves. If this caller doesn't have that `host_state`
        // then it probably means it was a host-created object like `Func::new`
        // which doesn't have any exports we want to return anyway.
        self.caller
            .host_state()
            .downcast_ref::<Instance>()?
            .get_export(&mut self.store, name)
    }

    /// Access the underlying data owned by this `Store`.
    ///
    /// Same as [`Store::data`](crate::Store::data)
    pub fn data(&self) -> &T {
        self.store.data()
    }

    /// Access the underlying data owned by this `Store`.
    ///
    /// Same as [`Store::data_mut`](crate::Store::data_mut)
    pub fn data_mut(&mut self) -> &mut T {
        self.store.data_mut()
    }

    /// Returns the underlying [`Engine`] this store is connected to.
    pub fn engine(&self) -> &Engine {
        self.store.engine()
    }

    /// Perform garbage collection.
    ///
    /// Same as [`Store::gc`](crate::Store::gc).
    #[cfg(feature = "gc")]
    pub fn gc(&mut self) {
        self.store.gc()
    }

    /// Perform garbage collection asynchronously.
    ///
    /// Same as [`Store::gc_async`](crate::Store::gc_async).
    #[cfg(all(feature = "async", feature = "gc"))]
    pub async fn gc_async(&mut self)
    where
        T: Send,
    {
        self.store.gc_async().await;
    }

    /// Returns the remaining fuel in the store.
    ///
    /// For more information see [`Store::get_fuel`](crate::Store::get_fuel)
    pub fn get_fuel(&self) -> Result<u64> {
        self.store.get_fuel()
    }

    /// Set the amount of fuel in this store to be consumed when executing wasm code.
    ///
    /// For more information see [`Store::set_fuel`](crate::Store::set_fuel)
    pub fn set_fuel(&mut self, fuel: u64) -> Result<()> {
        self.store.set_fuel(fuel)
    }

    /// Configures this `Store` to yield while executing futures every N units of fuel.
    ///
    /// For more information see
    /// [`Store::fuel_async_yield_interval`](crate::Store::fuel_async_yield_interval)
    pub fn fuel_async_yield_interval(&mut self, interval: Option<u64>) -> Result<()> {
        self.store.fuel_async_yield_interval(interval)
    }
}

impl<T> AsContext for Caller<'_, T> {
    type Data = T;
    fn as_context(&self) -> StoreContext<'_, T> {
        self.store.as_context()
    }
}

impl<T> AsContextMut for Caller<'_, T> {
    fn as_context_mut(&mut self) -> StoreContextMut<'_, T> {
        self.store.as_context_mut()
    }
}

// State stored inside a `VMNativeCallHostFuncContext`.
struct HostFuncState<F> {
    // The actual host function.
    func: F,

    // NB: We have to keep our `VMSharedTypeIndex` registered in the engine for
    // as long as this function exists.
    #[allow(dead_code)]
    ty: RegisteredType,
}

macro_rules! impl_into_func {
    ($num:tt $($args:ident)*) => {
        // Implement for functions without a leading `&Caller` parameter,
        // delegating to the implementation below which does have the leading
        // `Caller` parameter.
        #[allow(non_snake_case)]
        impl<T, F, $($args,)* R> IntoFunc<T, ($($args,)*), R> for F
        where
            F: Fn($($args),*) -> R + Send + Sync + 'static,
            $($args: WasmTy,)*
            R: WasmRet,
        {
            fn into_func(self, engine: &Engine) -> HostContext {
                let f = move |_: Caller<'_, T>, $($args:$args),*| {
                    self($($args),*)
                };

                f.into_func(engine)
            }
        }

        #[allow(non_snake_case)]
        impl<T, F, $($args,)* R> IntoFunc<T, (Caller<'_, T>, $($args,)*), R> for F
        where
            F: Fn(Caller<'_, T>, $($args),*) -> R + Send + Sync + 'static,
            $($args: WasmTy,)*
            R: WasmRet,
        {
            fn into_func(self, engine: &Engine) -> HostContext {
                /// This shim is a regular, non-closure function we can stuff
                /// inside `VMFuncRef::native_call`.
                ///
                /// It reads the actual callee closure out of
                /// `VMNativeCallHostFuncContext::host_state`, forwards
                /// arguments to that function, and finally forwards the results
                /// back out to the caller. It also handles traps and panics
                /// along the way.
                unsafe extern "C" fn native_call_shim<T, F, $($args,)* R>(
                    vmctx: *mut VMOpaqueContext,
                    caller_vmctx: *mut VMOpaqueContext,
                    $( $args: $args::Abi, )*
                    retptr: R::Retptr,
                ) -> R::Abi
                where
                    F: Fn(Caller<'_, T>, $( $args ),*) -> R + 'static,
                    $( $args: WasmTy, )*
                    R: WasmRet,
                {
                    // Note that this function is intentionally scoped into a
                    // separate closure. Handling traps and panics will involve
                    // longjmp-ing from this function which means we won't run
                    // destructors. As a result anything requiring a destructor
                    // should be part of this closure, and the long-jmp-ing
                    // happens after the closure in handling the result.
                    let run = move |mut caller: Caller<'_, T>| {
                        let vmctx = VMNativeCallHostFuncContext::from_opaque(vmctx);
                        let state = (*vmctx).host_state();

                        // Double-check ourselves in debug mode, but we control
                        // the `Any` here so an unsafe downcast should also
                        // work.
                        debug_assert!(state.is::<HostFuncState<F>>());
                        let state = &*(state as *const _ as *const HostFuncState<F>);
                        let func = &state.func;

                        let ret = 'ret: {
                            if let Err(trap) = caller.store.0.call_hook(CallHook::CallingHost) {
                                break 'ret R::fallible_from_error(trap);
                            }

                            let mut store = AutoAssertNoGc::new(caller.store.0);
                            $(let $args = $args::from_abi($args, &mut store);)*
                            let _ = &mut store;
                            drop(store);

                            let r = func(
                                caller.sub_caller(),
                                $( $args, )*
                            );
                            if let Err(trap) = caller.store.0.call_hook(CallHook::ReturningFromHost) {
                                break 'ret R::fallible_from_error(trap);
                            }
                            r.into_fallible()
                        };

                        if !ret.compatible_with_store(caller.store.0) {
                            bail!("host function attempted to return cross-`Store` value to Wasm")
                        } else {
                            let mut store = AutoAssertNoGc::new(&mut **caller.store.0);
                            let ret = ret.into_abi_for_ret(&mut store, retptr)?;
                            Ok(ret)
                        }
                    };

                    // With nothing else on the stack move `run` into this
                    // closure and then run it as part of `Caller::with`.
                    let result = crate::runtime::vm::catch_unwind_and_longjmp(move || {
                        let caller_vmctx = VMContext::from_opaque(caller_vmctx);
                        Caller::with(caller_vmctx, run)
                    });

                    match result {
                        Ok(val) => val,
                        Err(err) => crate::trap::raise(err),
                    }
                }

                /// This trampoline allows host code to indirectly call the
                /// wrapped function (e.g. via `Func::call` on a `funcref` that
                /// happens to reference our wrapped function).
                ///
                /// It reads the arguments out of the incoming `args` array,
                /// calls the given function pointer, and then stores the result
                /// back into the `args` array.
                unsafe extern "C" fn array_call_trampoline<T, F, $($args,)* R>(
                    callee_vmctx: *mut VMOpaqueContext,
                    caller_vmctx: *mut VMOpaqueContext,
                    args: *mut ValRaw,
                    _args_len: usize
                )
                where
                    F: Fn(Caller<'_, T>, $( $args ),*) -> R + 'static,
                    $($args: WasmTy,)*
                    R: WasmRet,
                {
                    let mut _n = 0;
                    $(
                        debug_assert!(_n < _args_len);
                        let $args = $args::abi_from_raw(args.add(_n));
                        _n += 1;
                    )*

                    R::wrap_trampoline(args, |retptr| {
                        native_call_shim::<T, F, $( $args, )* R>(callee_vmctx, caller_vmctx, $( $args, )* retptr)
                    });
                }

                let ty = R::func_type(
                    engine,
                    None::<ValType>.into_iter()
                        $(.chain(Some($args::valtype())))*
                );
                let type_index = ty.type_index();

                let array_call = array_call_trampoline::<T, F, $($args,)* R>;
                let native_call = NonNull::new(native_call_shim::<T, F, $($args,)* R> as *mut _).unwrap();

                let ctx = unsafe {
                    VMNativeCallHostFuncContext::new(
                        VMFuncRef {
                            native_call,
                            array_call,
                            wasm_call: None,
                            type_index,
                            vmctx: ptr::null_mut(),
                        },
                        Box::new(HostFuncState {
                            func: self,
                            ty: ty.into_registered_type(),
                        }),
                    )
                };

                ctx.into()
            }
        }
    }
}

for_each_function_signature!(impl_into_func);

#[doc(hidden)]
pub enum HostContext {
    Native(StoreBox<VMNativeCallHostFuncContext>),
    Array(StoreBox<VMArrayCallHostFuncContext>),
}

impl From<StoreBox<VMNativeCallHostFuncContext>> for HostContext {
    fn from(ctx: StoreBox<VMNativeCallHostFuncContext>) -> Self {
        HostContext::Native(ctx)
    }
}

impl From<StoreBox<VMArrayCallHostFuncContext>> for HostContext {
    fn from(ctx: StoreBox<VMArrayCallHostFuncContext>) -> Self {
        HostContext::Array(ctx)
    }
}

/// Representation of a host-defined function.
///
/// This is used for `Func::new` but also for `Linker`-defined functions. For
/// `Func::new` this is stored within a `Store`, and for `Linker`-defined
/// functions they wrap this up in `Arc` to enable shared ownership of this
/// across many stores.
///
/// Technically this structure needs a `<T>` type parameter to connect to the
/// `Store<T>` itself, but that's an unsafe contract of using this for now
/// rather than part of the struct type (to avoid `Func<T>` in the API).
pub(crate) struct HostFunc {
    ctx: HostContext,

    // Stored to unregister this function's signature with the engine when this
    // is dropped.
    engine: Engine,
}

impl HostFunc {
    /// Analog of [`Func::new`]
    ///
    /// # Panics
    ///
    /// Panics if the given function type is not associated with the given
    /// engine.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn new<T>(
        engine: &Engine,
        ty: FuncType,
        func: impl Fn(Caller<'_, T>, &[Val], &mut [Val]) -> Result<()> + Send + Sync + 'static,
    ) -> Self {
        assert!(ty.comes_from_same_engine(engine));
        let ty_clone = ty.clone();
        unsafe {
            HostFunc::new_unchecked(engine, ty, move |caller, values| {
                Func::invoke_host_func_for_wasm(caller, &ty_clone, values, &func)
            })
        }
    }

    /// Analog of [`Func::new_unchecked`]
    ///
    /// # Panics
    ///
    /// Panics if the given function type is not associated with the given
    /// engine.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub unsafe fn new_unchecked<T>(
        engine: &Engine,
        ty: FuncType,
        func: impl Fn(Caller<'_, T>, &mut [ValRaw]) -> Result<()> + Send + Sync + 'static,
    ) -> Self {
        assert!(ty.comes_from_same_engine(engine));
        let func = move |caller_vmctx, values: &mut [ValRaw]| {
            Caller::<T>::with(caller_vmctx, |mut caller| {
                caller.store.0.call_hook(CallHook::CallingHost)?;
                let result = func(caller.sub_caller(), values)?;
                caller.store.0.call_hook(CallHook::ReturningFromHost)?;
                Ok(result)
            })
        };
        let ctx = crate::trampoline::create_array_call_function(&ty, func, engine)
            .expect("failed to create function");
        HostFunc::_new(engine, ctx.into())
    }

    /// Analog of [`Func::wrap`]
    pub fn wrap<T, Params, Results>(
        engine: &Engine,
        func: impl IntoFunc<T, Params, Results>,
    ) -> Self {
        let ctx = func.into_func(engine);
        HostFunc::_new(engine, ctx)
    }

    /// Requires that this function's signature is already registered within
    /// `Engine`. This happens automatically during the above two constructors.
    fn _new(engine: &Engine, ctx: HostContext) -> Self {
        HostFunc {
            ctx,
            engine: engine.clone(),
        }
    }

    /// Inserts this `HostFunc` into a `Store`, returning the `Func` pointing to
    /// it.
    ///
    /// # Unsafety
    ///
    /// Can only be inserted into stores with a matching `T` relative to when
    /// this `HostFunc` was first created.
    pub unsafe fn to_func(self: &Arc<Self>, store: &mut StoreOpaque) -> Func {
        self.validate_store(store);
        let me = self.clone();
        Func::from_func_kind(FuncKind::SharedHost(me), store)
    }

    /// Inserts this `HostFunc` into a `Store`, returning the `Func` pointing to
    /// it.
    ///
    /// This function is similar to, but not equivalent, to `HostFunc::to_func`.
    /// Notably this function requires that the `Arc<Self>` pointer is otherwise
    /// rooted within the `StoreOpaque` via another means. When in doubt use
    /// `to_func` above as it's safer.
    ///
    /// # Unsafety
    ///
    /// Can only be inserted into stores with a matching `T` relative to when
    /// this `HostFunc` was first created.
    ///
    /// Additionally the `&Arc<Self>` is not cloned in this function. Instead a
    /// raw pointer to `Self` is stored within the `Store` for this function.
    /// The caller must arrange for the `Arc<Self>` to be "rooted" in the store
    /// provided via another means, probably by pushing to
    /// `StoreOpaque::rooted_host_funcs`.
    ///
    /// Similarly, the caller must arrange for `rooted_func_ref` to be rooted in
    /// the same store.
    pub unsafe fn to_func_store_rooted(
        self: &Arc<Self>,
        store: &mut StoreOpaque,
        rooted_func_ref: Option<NonNull<VMFuncRef>>,
    ) -> Func {
        self.validate_store(store);

        if rooted_func_ref.is_some() {
            debug_assert!(self.func_ref().wasm_call.is_none());
            debug_assert!(matches!(self.ctx, HostContext::Native(_)));
        }

        Func::from_func_kind(
            FuncKind::RootedHost(RootedHostFunc::new(self, rooted_func_ref)),
            store,
        )
    }

    /// Same as [`HostFunc::to_func`], different ownership.
    unsafe fn into_func(self, store: &mut StoreOpaque) -> Func {
        self.validate_store(store);
        Func::from_func_kind(FuncKind::Host(Box::new(self)), store)
    }

    fn validate_store(&self, store: &mut StoreOpaque) {
        // This assert is required to ensure that we can indeed safely insert
        // `self` into the `store` provided, otherwise the type information we
        // have listed won't be correct. This is possible to hit with the public
        // API of Wasmtime, and should be documented in relevant functions.
        assert!(
            Engine::same(&self.engine, store.engine()),
            "cannot use a store with a different engine than a linker was created with",
        );
    }

    pub(crate) fn sig_index(&self) -> VMSharedTypeIndex {
        self.func_ref().type_index
    }

    pub(crate) fn func_ref(&self) -> &VMFuncRef {
        match &self.ctx {
            HostContext::Native(ctx) => unsafe { (*ctx.get()).func_ref() },
            HostContext::Array(ctx) => unsafe { (*ctx.get()).func_ref() },
        }
    }

    pub(crate) fn host_ctx(&self) -> &HostContext {
        &self.ctx
    }

    fn export_func(&self) -> ExportFunction {
        ExportFunction {
            func_ref: NonNull::from(self.func_ref()),
        }
    }
}

impl FuncData {
    #[inline]
    fn export(&self) -> ExportFunction {
        self.kind.export()
    }

    pub(crate) fn sig_index(&self) -> VMSharedTypeIndex {
        unsafe { self.export().func_ref.as_ref().type_index }
    }
}

impl FuncKind {
    #[inline]
    fn export(&self) -> ExportFunction {
        match self {
            FuncKind::StoreOwned { export, .. } => *export,
            FuncKind::SharedHost(host) => host.export_func(),
            FuncKind::RootedHost(rooted) => ExportFunction {
                func_ref: NonNull::from(rooted.func_ref()),
            },
            FuncKind::Host(host) => host.export_func(),
        }
    }
}

use self::rooted::*;

/// An inner module is used here to force unsafe construction of
/// `RootedHostFunc` instead of accidentally safely allowing access to its
/// constructor.
mod rooted {
    use super::HostFunc;
    use crate::runtime::vm::{SendSyncPtr, VMFuncRef};
    use alloc::sync::Arc;
    use core::ptr::NonNull;

    /// A variant of a pointer-to-a-host-function used in `FuncKind::RootedHost`
    /// above.
    ///
    /// For more documentation see `FuncKind::RootedHost`, `InstancePre`, and
    /// `HostFunc::to_func_store_rooted`.
    pub(crate) struct RootedHostFunc {
        func: SendSyncPtr<HostFunc>,
        func_ref: Option<SendSyncPtr<VMFuncRef>>,
    }

    impl RootedHostFunc {
        /// Note that this is `unsafe` because this wrapper type allows safe
        /// access to the pointer given at any time, including outside the
        /// window of validity of `func`, so callers must not use the return
        /// value past the lifetime of the provided `func`.
        ///
        /// Similarly, callers must ensure that the given `func_ref` is valid
        /// for the lifetime of the return value.
        pub(crate) unsafe fn new(
            func: &Arc<HostFunc>,
            func_ref: Option<NonNull<VMFuncRef>>,
        ) -> RootedHostFunc {
            RootedHostFunc {
                func: NonNull::from(&**func).into(),
                func_ref: func_ref.map(|p| p.into()),
            }
        }

        pub(crate) fn func(&self) -> &HostFunc {
            // Safety invariants are upheld by the `RootedHostFunc::new` caller.
            unsafe { self.func.as_ref() }
        }

        pub(crate) fn func_ref(&self) -> &VMFuncRef {
            if let Some(f) = self.func_ref {
                // Safety invariants are upheld by the `RootedHostFunc::new` caller.
                unsafe { f.as_ref() }
            } else {
                self.func().func_ref()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;

    #[test]
    fn hash_key_is_stable_across_duplicate_store_data_entries() -> Result<()> {
        let mut store = Store::<()>::default();
        let module = Module::new(
            store.engine(),
            r#"
                (module
                    (func (export "f")
                        nop
                    )
                )
            "#,
        )?;
        let instance = Instance::new(&mut store, &module, &[])?;

        // Each time we `get_func`, we call `Func::from_wasmtime` which adds a
        // new entry to `StoreData`, so `f1` and `f2` will have different
        // indices into `StoreData`.
        let f1 = instance.get_func(&mut store, "f").unwrap();
        let f2 = instance.get_func(&mut store, "f").unwrap();

        // But their hash keys are the same.
        assert!(
            f1.hash_key(&mut store.as_context_mut().0)
                == f2.hash_key(&mut store.as_context_mut().0)
        );

        // But the hash keys are different from different funcs.
        let instance2 = Instance::new(&mut store, &module, &[])?;
        let f3 = instance2.get_func(&mut store, "f").unwrap();
        assert!(
            f1.hash_key(&mut store.as_context_mut().0)
                != f3.hash_key(&mut store.as_context_mut().0)
        );

        Ok(())
    }
}
