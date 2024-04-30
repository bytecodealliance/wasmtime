use crate::component::instance::{Instance, InstanceData};
use crate::component::storage::storage_as_slice;
use crate::component::types::Type;
use crate::component::values::Val;
use crate::runtime::vm::component::ResourceTables;
use crate::runtime::vm::{Export, ExportFunction};
use crate::store::{StoreOpaque, Stored};
use crate::{AsContext, AsContextMut, StoreContextMut, ValRaw};
use anyhow::{bail, Context, Result};
use std::mem::{self, MaybeUninit};
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::component::{
    CanonicalOptions, ComponentTypes, CoreDef, InterfaceType, RuntimeComponentInstanceIndex,
    TypeFuncIndex, TypeTuple, MAX_FLAT_PARAMS, MAX_FLAT_RESULTS,
};

/// A helper macro to safely map `MaybeUninit<T>` to `MaybeUninit<U>` where `U`
/// is a field projection within `T`.
///
/// This is intended to be invoked as:
///
/// ```ignore
/// struct MyType {
///     field: u32,
/// }
///
/// let initial: &mut MaybeUninit<MyType> = ...;
/// let field: &mut MaybeUninit<u32> = map_maybe_uninit!(initial.field);
/// ```
///
/// Note that array accesses are also supported:
///
/// ```ignore
///
/// let initial: &mut MaybeUninit<[u32; 2]> = ...;
/// let element: &mut MaybeUninit<u32> = map_maybe_uninit!(initial[1]);
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! map_maybe_uninit {
    ($maybe_uninit:ident $($field:tt)*) => ({
        #[allow(unused_unsafe)]
        {
            unsafe {
                use $crate::component::__internal::MaybeUninitExt;

                let m: &mut std::mem::MaybeUninit<_> = $maybe_uninit;
                // Note the usage of `addr_of_mut!` here which is an attempt to "stay
                // safe" here where we never accidentally create `&mut T` where `T` is
                // actually uninitialized, hopefully appeasing the Rust unsafe
                // guidelines gods.
                m.map(|p| std::ptr::addr_of_mut!((*p)$($field)*))
            }
        }
    })
}

#[doc(hidden)]
pub trait MaybeUninitExt<T> {
    /// Maps `MaybeUninit<T>` to `MaybeUninit<U>` using the closure provided.
    ///
    /// Note that this is `unsafe` as there is no guarantee that `U` comes from
    /// `T`.
    unsafe fn map<U>(&mut self, f: impl FnOnce(*mut T) -> *mut U) -> &mut MaybeUninit<U>;
}

impl<T> MaybeUninitExt<T> for MaybeUninit<T> {
    unsafe fn map<U>(&mut self, f: impl FnOnce(*mut T) -> *mut U) -> &mut MaybeUninit<U> {
        let new_ptr = f(self.as_mut_ptr());
        std::mem::transmute::<*mut U, &mut MaybeUninit<U>>(new_ptr)
    }
}

mod host;
mod options;
mod typed;
pub use self::host::*;
pub use self::options::*;
pub use self::typed::*;

#[repr(C)]
union ParamsAndResults<Params: Copy, Return: Copy> {
    params: Params,
    ret: Return,
}

/// A WebAssembly component function which can be called.
///
/// This type is the dual of [`wasmtime::Func`](crate::Func) for component
/// functions. An instance of [`Func`] represents a component function from a
/// component [`Instance`](crate::component::Instance). Like with
/// [`wasmtime::Func`](crate::Func) it's possible to call functions either
/// synchronously or asynchronously and either typed or untyped.
#[derive(Copy, Clone, Debug)]
pub struct Func(Stored<FuncData>);

#[doc(hidden)]
pub struct FuncData {
    export: ExportFunction,
    ty: TypeFuncIndex,
    types: Arc<ComponentTypes>,
    options: Options,
    instance: Instance,
    component_instance: RuntimeComponentInstanceIndex,
    post_return: Option<ExportFunction>,
    post_return_arg: Option<ValRaw>,
}

impl Func {
    pub(crate) fn from_lifted_func(
        store: &mut StoreOpaque,
        instance: &Instance,
        data: &InstanceData,
        ty: TypeFuncIndex,
        func: &CoreDef,
        options: &CanonicalOptions,
    ) -> Func {
        let export = match data.lookup_def(store, func) {
            Export::Function(f) => f,
            _ => unreachable!(),
        };
        let memory = options
            .memory
            .map(|i| NonNull::new(data.instance().runtime_memory(i)).unwrap());
        let realloc = options.realloc.map(|i| data.instance().runtime_realloc(i));
        let post_return = options.post_return.map(|i| {
            let func_ref = data.instance().runtime_post_return(i);
            ExportFunction { func_ref }
        });
        let component_instance = options.instance;
        let options = unsafe { Options::new(store.id(), memory, realloc, options.string_encoding) };
        Func(store.store_data_mut().insert(FuncData {
            export,
            options,
            ty,
            types: data.component_types().clone(),
            instance: *instance,
            component_instance,
            post_return,
            post_return_arg: None,
        }))
    }

    /// Attempt to cast this [`Func`] to a statically typed [`TypedFunc`] with
    /// the provided `Params` and `Return`.
    ///
    /// This function will perform a type-check at runtime that the [`Func`]
    /// takes `Params` as parameters and returns `Return`. If the type-check
    /// passes then a [`TypedFunc`] will be returned which can be used to
    /// invoke the function in an efficient, statically-typed, and ergonomic
    /// manner.
    ///
    /// The `Params` type parameter here is a tuple of the parameters to the
    /// function. A function which takes no arguments should use `()`, a
    /// function with one argument should use `(T,)`, etc. Note that all
    /// `Params` must also implement the [`Lower`] trait since they're going
    /// into wasm.
    ///
    /// The `Return` type parameter is the return value of this function. A
    /// return value of `()` means that there's no return (similar to a Rust
    /// unit return) and otherwise a type `T` can be specified. Note that the
    /// `Return` must also implement the [`Lift`] trait since it's coming from
    /// wasm.
    ///
    /// Types specified here must implement the [`ComponentType`] trait. This
    /// trait is implemented for built-in types to Rust such as integer
    /// primitives, floats, `Option<T>`, `Result<T, E>`, strings, `Vec<T>`, and
    /// more. As parameters you'll be passing native Rust types.
    ///
    /// See the documentation for [`ComponentType`] for more information about
    /// supported types.
    ///
    /// # Errors
    ///
    /// If the function does not actually take `Params` as its parameters or
    /// return `Return` then an error will be returned.
    ///
    /// # Panics
    ///
    /// This function will panic if `self` is not owned by the `store`
    /// specified.
    ///
    /// # Examples
    ///
    /// Calling a function which takes no parameters and has no return value:
    ///
    /// ```
    /// # use wasmtime::component::Func;
    /// # use wasmtime::Store;
    /// # fn foo(func: &Func, store: &mut Store<()>) -> anyhow::Result<()> {
    /// let typed = func.typed::<(), ()>(&store)?;
    /// typed.call(store, ())?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Calling a function which takes one string parameter and returns a
    /// string:
    ///
    /// ```
    /// # use wasmtime::component::Func;
    /// # use wasmtime::Store;
    /// # fn foo(func: &Func, mut store: Store<()>) -> anyhow::Result<()> {
    /// let typed = func.typed::<(&str,), (String,)>(&store)?;
    /// let ret = typed.call(&mut store, ("Hello, ",))?.0;
    /// println!("returned string was: {}", ret);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Calling a function which takes multiple parameters and returns a boolean:
    ///
    /// ```
    /// # use wasmtime::component::Func;
    /// # use wasmtime::Store;
    /// # fn foo(func: &Func, mut store: Store<()>) -> anyhow::Result<()> {
    /// let typed = func.typed::<(u32, Option<&str>, &[u8]), (bool,)>(&store)?;
    /// let ok: bool = typed.call(&mut store, (1, Some("hello"), b"bytes!"))?.0;
    /// println!("return value was: {ok}");
    /// # Ok(())
    /// # }
    /// ```
    pub fn typed<Params, Return>(&self, store: impl AsContext) -> Result<TypedFunc<Params, Return>>
    where
        Params: ComponentNamedList + Lower,
        Return: ComponentNamedList + Lift,
    {
        self._typed(store.as_context().0, None)
    }

    pub(crate) fn _typed<Params, Return>(
        &self,
        store: &StoreOpaque,
        instance: Option<&InstanceData>,
    ) -> Result<TypedFunc<Params, Return>>
    where
        Params: ComponentNamedList + Lower,
        Return: ComponentNamedList + Lift,
    {
        self.typecheck::<Params, Return>(store, instance)?;
        unsafe { Ok(TypedFunc::new_unchecked(*self)) }
    }

    fn typecheck<Params, Return>(
        &self,
        store: &StoreOpaque,
        instance: Option<&InstanceData>,
    ) -> Result<()>
    where
        Params: ComponentNamedList + Lower,
        Return: ComponentNamedList + Lift,
    {
        let data = &store[self.0];
        let cx = instance
            .unwrap_or_else(|| &store[data.instance.0].as_ref().unwrap())
            .ty();
        let ty = &cx.types[data.ty];

        Params::typecheck(&InterfaceType::Tuple(ty.params), &cx)
            .context("type mismatch with parameters")?;
        Return::typecheck(&InterfaceType::Tuple(ty.results), &cx)
            .context("type mismatch with results")?;

        Ok(())
    }

    /// Get the parameter types for this function.
    pub fn params(&self, store: impl AsContext) -> Box<[Type]> {
        let store = store.as_context();
        let data = &store[self.0];
        let instance = store[data.instance.0].as_ref().unwrap();
        data.types[data.types[data.ty].params]
            .types
            .iter()
            .map(|ty| Type::from(ty, &instance.ty()))
            .collect()
    }

    /// Get the result types for this function.
    pub fn results(&self, store: impl AsContext) -> Box<[Type]> {
        let store = store.as_context();
        let data = &store[self.0];
        let instance = store[data.instance.0].as_ref().unwrap();
        data.types[data.types[data.ty].results]
            .types
            .iter()
            .map(|ty| Type::from(ty, &instance.ty()))
            .collect()
    }

    /// Invokes this function with the `params` given and returns the result.
    ///
    /// The `params` provided must match the parameters that this function takes
    /// in terms of their types and the number of parameters. Results will be
    /// written to the `results` slice provided if the call completes
    /// successfully. The initial types of the values in `results` are ignored
    /// and values are overwritten to write the result. It's required that the
    /// size of `results` exactly matches the number of results that this
    /// function produces.
    ///
    /// Note that after a function is invoked the embedder needs to invoke
    /// [`Func::post_return`] to execute any final cleanup required by the
    /// guest. This function call is required to either call the function again
    /// or to call another function.
    ///
    /// For more detailed information see the documentation of
    /// [`TypedFunc::call`].
    ///
    /// # Errors
    ///
    /// Returns an error in situations including but not limited to:
    ///
    /// * `params` is not the right size or if the values have the wrong type
    /// * `results` is not the right size
    /// * A trap occurs while executing the function
    /// * The function calls a host function which returns an error
    ///
    /// See [`TypedFunc::call`] for more information in addition to
    /// [`wasmtime::Func::call`](crate::Func::call).
    ///
    /// # Panics
    ///
    /// Panics if this is called on a function in an asyncronous store. This
    /// only works with functions defined within a synchronous store. Also
    /// panics if `store` does not own this function.
    pub fn call(
        &self,
        mut store: impl AsContextMut,
        params: &[Val],
        results: &mut [Val],
    ) -> Result<()> {
        let mut store = store.as_context_mut();
        assert!(
            !store.0.async_support(),
            "must use `call_async` when async support is enabled on the config"
        );
        self.call_impl(&mut store.as_context_mut(), params, results)
    }

    /// Exactly like [`Self::call`] except for use on async stores.
    ///
    /// Note that after this [`Func::post_return_async`] will be used instead of
    /// the synchronous version at [`Func::post_return`].
    ///
    /// # Panics
    ///
    /// Panics if this is called on a function in a synchronous store. This
    /// only works with functions defined within an asynchronous store. Also
    /// panics if `store` does not own this function.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
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
            "cannot use `call_async` without enabling async support in the config"
        );
        store
            .on_fiber(|store| self.call_impl(store, params, results))
            .await?
    }

    fn call_impl(
        &self,
        mut store: impl AsContextMut,
        params: &[Val],
        results: &mut [Val],
    ) -> Result<()> {
        let store = &mut store.as_context_mut();

        let param_tys = self.params(&store);
        let result_tys = self.results(&store);

        if param_tys.len() != params.len() {
            bail!(
                "expected {} argument(s), got {}",
                param_tys.len(),
                params.len()
            );
        }
        if result_tys.len() != results.len() {
            bail!(
                "expected {} results(s), got {}",
                result_tys.len(),
                results.len()
            );
        }

        self.call_raw(
            store,
            params,
            |cx, params, params_ty, dst: &mut MaybeUninit<[ValRaw; MAX_FLAT_PARAMS]>| {
                let params_ty = match params_ty {
                    InterfaceType::Tuple(i) => &cx.types[i],
                    _ => unreachable!(),
                };
                if params_ty.abi.flat_count(MAX_FLAT_PARAMS).is_some() {
                    let dst = &mut unsafe {
                        mem::transmute::<_, &mut [MaybeUninit<ValRaw>; MAX_FLAT_PARAMS]>(dst)
                    }
                    .iter_mut();

                    params
                        .iter()
                        .zip(params_ty.types.iter())
                        .try_for_each(|(param, ty)| param.lower(cx, *ty, dst))
                } else {
                    self.store_args(cx, &params_ty, params, dst)
                }
            },
            |cx, results_ty, src: &[ValRaw; MAX_FLAT_RESULTS]| {
                let results_ty = match results_ty {
                    InterfaceType::Tuple(i) => &cx.types[i],
                    _ => unreachable!(),
                };
                if results_ty.abi.flat_count(MAX_FLAT_RESULTS).is_some() {
                    let mut flat = src.iter();
                    for (ty, slot) in results_ty.types.iter().zip(results) {
                        *slot = Val::lift(cx, *ty, &mut flat)?;
                    }
                    Ok(())
                } else {
                    Self::load_results(cx, results_ty, results, &mut src.iter())
                }
            },
        )
    }

    /// Invokes the underlying wasm function, lowering arguments and lifting the
    /// result.
    ///
    /// The `lower` function and `lift` function provided here are what actually
    /// do the lowering and lifting. The `LowerParams` and `LowerReturn` types
    /// are what will be allocated on the stack for this function call. They
    /// should be appropriately sized for the lowering/lifting operation
    /// happening.
    fn call_raw<T, Params: ?Sized, Return, LowerParams, LowerReturn>(
        &self,
        store: &mut StoreContextMut<'_, T>,
        params: &Params,
        lower: impl FnOnce(
            &mut LowerContext<'_, T>,
            &Params,
            InterfaceType,
            &mut MaybeUninit<LowerParams>,
        ) -> Result<()>,
        lift: impl FnOnce(&mut LiftContext<'_>, InterfaceType, &LowerReturn) -> Result<Return>,
    ) -> Result<Return>
    where
        LowerParams: Copy,
        LowerReturn: Copy,
    {
        let FuncData {
            export,
            options,
            instance,
            component_instance,
            ty,
            ..
        } = store.0[self.0];

        let space = &mut MaybeUninit::<ParamsAndResults<LowerParams, LowerReturn>>::uninit();

        // Double-check the size/alignemnt of `space`, just in case.
        //
        // Note that this alone is not enough to guarantee the validity of the
        // `unsafe` block below, but it's definitely required. In any case LLVM
        // should be able to trivially see through these assertions and remove
        // them in release mode.
        let val_size = mem::size_of::<ValRaw>();
        let val_align = mem::align_of::<ValRaw>();
        assert!(mem::size_of_val(space) % val_size == 0);
        assert!(mem::size_of_val(map_maybe_uninit!(space.params)) % val_size == 0);
        assert!(mem::size_of_val(map_maybe_uninit!(space.ret)) % val_size == 0);
        assert!(mem::align_of_val(space) == val_align);
        assert!(mem::align_of_val(map_maybe_uninit!(space.params)) == val_align);
        assert!(mem::align_of_val(map_maybe_uninit!(space.ret)) == val_align);

        let instance = store.0[instance.0].as_ref().unwrap();
        let types = instance.component_types().clone();
        let mut flags = instance.instance().instance_flags(component_instance);

        unsafe {
            // Test the "may enter" flag which is a "lock" on this instance.
            // This is immediately set to `false` afterwards and note that
            // there's no on-cleanup setting this flag back to true. That's an
            // intentional design aspect where if anything goes wrong internally
            // from this point on the instance is considered "poisoned" and can
            // never be entered again. The only time this flag is set to `true`
            // again is after post-return logic has completed successfully.
            if !flags.may_enter() {
                bail!(crate::Trap::CannotEnterComponent);
            }
            flags.set_may_enter(false);

            debug_assert!(flags.may_leave());
            flags.set_may_leave(false);
            let instance_ptr = instance.instance_ptr();
            let mut cx = LowerContext::new(store.as_context_mut(), &options, &types, instance_ptr);
            cx.enter_call();
            let result = lower(
                &mut cx,
                params,
                InterfaceType::Tuple(types[ty].params),
                map_maybe_uninit!(space.params),
            );
            flags.set_may_leave(true);
            result?;

            // This is unsafe as we are providing the guarantee that all the
            // inputs are valid. The various pointers passed in for the function
            // are all valid since they're coming from our store, and the
            // `params_and_results` should have the correct layout for the core
            // wasm function we're calling. Note that this latter point relies
            // on the correctness of this module and `ComponentType`
            // implementations, hence `ComponentType` being an `unsafe` trait.
            crate::Func::call_unchecked_raw(
                store,
                export.func_ref,
                space.as_mut_ptr().cast(),
                mem::size_of_val(space) / mem::size_of::<ValRaw>(),
            )?;

            // Note that `.assume_init_ref()` here is unsafe but we're relying
            // on the correctness of the structure of `LowerReturn` and the
            // type-checking performed to acquire the `TypedFunc` to make this
            // safe. It should be the case that `LowerReturn` is the exact
            // representation of the return value when interpreted as
            // `[ValRaw]`, and additionally they should have the correct types
            // for the function we just called (which filled in the return
            // values).
            let ret = map_maybe_uninit!(space.ret).assume_init_ref();

            // Lift the result into the host while managing post-return state
            // here as well.
            //
            // After a successful lift the return value of the function, which
            // is currently required to be 0 or 1 values according to the
            // canonical ABI, is saved within the `Store`'s `FuncData`. This'll
            // later get used in post-return.
            flags.set_needs_post_return(true);
            let val = lift(
                &mut LiftContext::new(store.0, &options, &types, instance_ptr),
                InterfaceType::Tuple(types[ty].results),
                ret,
            )?;
            let ret_slice = storage_as_slice(ret);
            let data = &mut store.0[self.0];
            assert!(data.post_return_arg.is_none());
            match ret_slice.len() {
                0 => data.post_return_arg = Some(ValRaw::i32(0)),
                1 => data.post_return_arg = Some(ret_slice[0]),
                _ => unreachable!(),
            }
            return Ok(val);
        }
    }

    /// Invokes the `post-return` canonical ABI option, if specified, after a
    /// [`Func::call`] has finished.
    ///
    /// This function is a required method call after a [`Func::call`] completes
    /// successfully. After the embedder has finished processing the return
    /// value then this function must be invoked.
    ///
    /// # Errors
    ///
    /// This function will return an error in the case of a WebAssembly trap
    /// happening during the execution of the `post-return` function, if
    /// specified.
    ///
    /// # Panics
    ///
    /// This function will panic if it's not called under the correct
    /// conditions. This can only be called after a previous invocation of
    /// [`Func::call`] completes successfully, and this function can only
    /// be called for the same [`Func`] that was `call`'d.
    ///
    /// If this function is called when [`Func::call`] was not previously
    /// called, then it will panic. If a different [`Func`] for the same
    /// component instance was invoked then this function will also panic
    /// because the `post-return` needs to happen for the other function.
    ///
    /// Panics if this is called on a function in an asynchronous store.
    /// This only works with functions defined within a synchronous store.
    #[inline]
    pub fn post_return(&self, mut store: impl AsContextMut) -> Result<()> {
        let store = store.as_context_mut();
        assert!(
            !store.0.async_support(),
            "must use `post_return_async` when async support is enabled on the config"
        );
        self.post_return_impl(store)
    }

    /// Exactly like [`Self::post_return`] except for use on async stores.
    ///
    /// # Panics
    ///
    /// Panics if this is called on a function in a synchronous store. This
    /// only works with functions defined within an asynchronous store.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub async fn post_return_async<T: Send>(
        &self,
        mut store: impl AsContextMut<Data = T>,
    ) -> Result<()> {
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "cannot use `call_async` without enabling async support in the config"
        );
        // Future optimization opportunity: conditionally use a fiber here since
        // some func's post_return will not need the async context (i.e. end up
        // calling async host functionality)
        store.on_fiber(|store| self.post_return_impl(store)).await?
    }

    fn post_return_impl(&self, mut store: impl AsContextMut) -> Result<()> {
        let mut store = store.as_context_mut();
        let data = &mut store.0[self.0];
        let instance = data.instance;
        let post_return = data.post_return;
        let component_instance = data.component_instance;
        let post_return_arg = data.post_return_arg.take();
        let instance = store.0[instance.0].as_ref().unwrap().instance_ptr();

        unsafe {
            let mut flags = (*instance).instance_flags(component_instance);

            // First assert that the instance is in a "needs post return" state.
            // This will ensure that the previous action on the instance was a
            // function call above. This flag is only set after a component
            // function returns so this also can't be called (as expected)
            // during a host import for example.
            //
            // Note, though, that this assert is not sufficient because it just
            // means some function on this instance needs its post-return
            // called. We need a precise post-return for a particular function
            // which is the second assert here (the `.expect`). That will assert
            // that this function itself needs to have its post-return called.
            //
            // The theory at least is that these two asserts ensure component
            // model semantics are upheld where the host properly calls
            // `post_return` on the right function despite the call being a
            // separate step in the API.
            assert!(
                flags.needs_post_return(),
                "post_return can only be called after a function has previously been called",
            );
            let post_return_arg = post_return_arg.expect("calling post_return on wrong function");

            // This is a sanity-check assert which shouldn't ever trip.
            assert!(!flags.may_enter());

            // Unset the "needs post return" flag now that post-return is being
            // processed. This will cause future invocations of this method to
            // panic, even if the function call below traps.
            flags.set_needs_post_return(false);

            // If the function actually had a `post-return` configured in its
            // canonical options that's executed here.
            //
            // Note that if this traps (returns an error) this function
            // intentionally leaves the instance in a "poisoned" state where it
            // can no longer be entered because `may_enter` is `false`.
            if let Some(func) = post_return {
                crate::Func::call_unchecked_raw(
                    &mut store,
                    func.func_ref,
                    &post_return_arg as *const ValRaw as *mut ValRaw,
                    1,
                )?;
            }

            // And finally if everything completed successfully then the "may
            // enter" flag is set to `true` again here which enables further use
            // of the component.
            flags.set_may_enter(true);

            let (calls, host_table, _) = store.0.component_resource_state();
            ResourceTables {
                calls,
                host_table: Some(host_table),
                tables: Some((*instance).component_resource_tables()),
            }
            .exit_call()?;
        }
        Ok(())
    }

    fn store_args<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        params_ty: &TypeTuple,
        args: &[Val],
        dst: &mut MaybeUninit<[ValRaw; MAX_FLAT_PARAMS]>,
    ) -> Result<()> {
        let size = usize::try_from(params_ty.abi.size32).unwrap();
        let ptr = cx.realloc(0, 0, params_ty.abi.align32, size)?;
        let mut offset = ptr;
        for (ty, arg) in params_ty.types.iter().zip(args) {
            let abi = cx.types.canonical_abi(ty);
            arg.store(cx, *ty, abi.next_field32_size(&mut offset))?;
        }

        map_maybe_uninit!(dst[0]).write(ValRaw::i64(ptr as i64));

        Ok(())
    }

    fn load_results(
        cx: &mut LiftContext<'_>,
        results_ty: &TypeTuple,
        results: &mut [Val],
        src: &mut std::slice::Iter<'_, ValRaw>,
    ) -> Result<()> {
        // FIXME: needs to read an i64 for memory64
        let ptr = usize::try_from(src.next().unwrap().get_u32())?;
        if ptr % usize::try_from(results_ty.abi.align32)? != 0 {
            bail!("return pointer not aligned");
        }

        let bytes = cx
            .memory()
            .get(ptr..)
            .and_then(|b| b.get(..usize::try_from(results_ty.abi.size32).unwrap()))
            .ok_or_else(|| anyhow::anyhow!("pointer out of bounds of memory"))?;

        let mut offset = 0;
        for (ty, slot) in results_ty.types.iter().zip(results) {
            let abi = cx.types.canonical_abi(ty);
            let offset = abi.next_field32_size(&mut offset);
            *slot = Val::load(cx, *ty, &bytes[offset..][..abi.size32 as usize])?;
        }
        Ok(())
    }
}
