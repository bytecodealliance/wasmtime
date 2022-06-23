use crate::component::func::{
    Func, Memory, MemoryMut, Options, MAX_STACK_PARAMS, MAX_STACK_RESULTS,
};
use crate::store::StoreOpaque;
use crate::{AsContext, AsContextMut, StoreContext, StoreContextMut, ValRaw};
use anyhow::{bail, Result};
use std::borrow::Cow;
use std::marker;
use std::mem::{self, MaybeUninit};
use std::str;
use wasmtime_environ::component::{ComponentTypes, InterfaceType, StringEncoding};

/// A statically-typed version of [`Func`] which takes `Params` as input and
/// returns `Return`.
///
/// This is an efficient way to invoke a WebAssembly component where if the
/// inputs and output are statically known this can eschew the vast majority of
/// machinery and checks when calling WebAssembly. This is the most optimized
/// way to call a WebAssembly component.
///
/// Note that like [`Func`] this is a pointer within a [`Store`](crate::Store)
/// and usage will panic if used with the wrong store.
///
/// This type is primarily created with the [`Func::typed`] API.
pub struct TypedFunc<Params, Return> {
    func: Func,

    // The definition of this field is somewhat subtle and may be surprising.
    // Naively one might expect something like
    //
    //      _marker: marker::PhantomData<fn(Params) -> Return>,
    //
    // Since this is a function pointer after all. The problem with this
    // definition though is that it imposes the wrong variance on `Params` from
    // what we want. Abstractly a `fn(Params)` is able to store `Params` within
    // it meaning you can only give it `Params` that live longer than the
    // function pointer.
    //
    // With a component model function, however, we're always copying data from
    // the host into the guest, so we are never storing pointers to `Params`
    // into the guest outside the duration of a `call`, meaning we can actually
    // accept values in `TypedFunc::call` which live for a shorter duration
    // than the `Params` argument on the struct.
    //
    // This all means that we don't use a phantom function pointer, but instead
    // feign phantom storage here to get the variance desired.
    _marker: marker::PhantomData<(Params, Return)>,
}

impl<Params, Return> Copy for TypedFunc<Params, Return> {}

impl<Params, Return> Clone for TypedFunc<Params, Return> {
    fn clone(&self) -> TypedFunc<Params, Return> {
        *self
    }
}

impl<Params, Return> TypedFunc<Params, Return>
where
    Params: ComponentParams + Lower,
    Return: Lift,
{
    /// Creates a new [`TypedFunc`] from the provided component [`Func`],
    /// unsafely asserting that the underlying function takes `Params` as
    /// input and returns `Return`.
    ///
    /// # Unsafety
    ///
    /// This is an unsafe function because it does not verify that the [`Func`]
    /// provided actually implements this signature. It's up to the caller to
    /// have performed some other sort of check to ensure that the signature is
    /// correct.
    pub unsafe fn new_unchecked(func: Func) -> TypedFunc<Params, Return> {
        TypedFunc {
            _marker: marker::PhantomData,
            func,
        }
    }

    /// Returns the underlying un-typed [`Func`] that this [`TypedFunc`]
    /// references.
    pub fn func(&self) -> &Func {
        &self.func
    }

    /// Calls the underlying WebAssembly component function using the provided
    /// `params` as input.
    ///
    /// This method is used to enter into a component. Execution happens within
    /// the `store` provided. The `params` are copied into WebAssembly memory
    /// as appropriate and a core wasm function is invoked.
    ///
    /// # Post-return
    ///
    /// In the component model each function can have a "post return" specified
    /// which allows cleaning up the arguments returned to the host. For example
    /// if WebAssembly returns a string to the host then it might be a uniquely
    /// allocated string which, after the host finishes processing it, needs to
    /// be deallocated in the wasm instance's own linear memory to prevent
    /// memory leaks in wasm itself. The `post-return` canonical abi option is
    /// used to configured this.
    ///
    /// To accommodate this feature of the component model after invoking a
    /// function via [`TypedFunc::call`] you must next invoke
    /// [`TypedFunc::post_return`]. Note that the return value of the function
    /// should be processed between these two function calls. The return value
    /// continues to be usable from an embedder's perspective after
    /// `post_return` is called, but after `post_return` is invoked it may no
    /// longer retain the same value that the wasm module originally returned.
    ///
    /// Also note that [`TypedFunc::post_return`] must be invoked irrespective
    /// of whether the canonical ABI option `post-return` was configured or not.
    /// This means that embedders must unconditionally call
    /// [`TypedFunc::post_return`] when a function returns. If this function
    /// call returns an error, however, then [`TypedFunc::post_return`] is not
    /// required.
    ///
    /// # Errors
    ///
    /// This function can return an error for a number of reasons:
    ///
    /// * If the wasm itself traps during execution.
    /// * If the wasm traps while copying arguments into memory.
    /// * If the wasm provides bad allocation pointers when copying arguments
    ///   into memory.
    /// * If the wasm returns a value which violates the canonical ABI.
    /// * If this function's instances cannot be entered, for example if the
    ///   instance is currently calling a host function.
    /// * If a previous function call occurred and the corresponding
    ///   `post_return` hasn't been invoked yet.
    ///
    /// In general there are many ways that things could go wrong when copying
    /// types in and out of a wasm module with the canonical ABI, and certain
    /// error conditions are specific to certain types. For example a
    /// WebAssembly module can't return an invalid `char`. When allocating space
    /// for this host to copy a string into the returned pointer must be
    /// in-bounds in memory.
    ///
    /// If an error happens then the error should contain detailed enough
    /// information to understand which part of the canonical ABI went wrong
    /// and what to inspect.
    ///
    /// # Panics
    ///
    /// This function will panic if `store` does not own this function.
    pub fn call(&self, mut store: impl AsContextMut, params: Params) -> Result<Return> {
        let store = &mut store.as_context_mut();
        // Note that this is in theory simpler than it might read at this time.
        // Here we're doing a runtime dispatch on the `flatten_count` for the
        // params/results to see whether they're inbounds. This creates 4 cases
        // to handle. In reality this is a highly optimizable branch where LLVM
        // will easily figure out that only one branch here is taken.
        //
        // Otherwise this current construction is done to ensure that the stack
        // space reserved for the params/results is always of the appropriate
        // size (as the params/results needed differ depending on the "flatten"
        // count)
        if Params::flatten_count() <= MAX_STACK_PARAMS {
            if Return::flatten_count() <= MAX_STACK_RESULTS {
                self.call_raw(
                    store,
                    &params,
                    Self::lower_stack_args,
                    Self::lift_stack_result,
                )
            } else {
                self.call_raw(
                    store,
                    &params,
                    Self::lower_stack_args,
                    Self::lift_heap_result,
                )
            }
        } else {
            if Return::flatten_count() <= MAX_STACK_RESULTS {
                self.call_raw(
                    store,
                    &params,
                    Self::lower_heap_args,
                    Self::lift_stack_result,
                )
            } else {
                self.call_raw(
                    store,
                    &params,
                    Self::lower_heap_args,
                    Self::lift_heap_result,
                )
            }
        }
    }

    /// Lower parameters directly onto the stack specified by the `dst`
    /// location.
    ///
    /// This is only valid to call when the "flatten count" is small enough, or
    /// when the canonical ABI says arguments go through the stack rather than
    /// the heap.
    fn lower_stack_args<T>(
        store: &mut StoreContextMut<'_, T>,
        options: &Options,
        params: &Params,
        dst: &mut MaybeUninit<Params::Lower>,
    ) -> Result<()> {
        assert!(Params::flatten_count() <= MAX_STACK_PARAMS);
        params.lower(store, options, dst)?;
        Ok(())
    }

    /// Lower parameters onto a heap-allocated location.
    ///
    /// This is used when the stack space to be used for the arguments is above
    /// the `MAX_STACK_PARAMS` threshold. Here the wasm's `realloc` function is
    /// invoked to allocate space and then parameters are stored at that heap
    /// pointer location.
    fn lower_heap_args<T>(
        store: &mut StoreContextMut<'_, T>,
        options: &Options,
        params: &Params,
        dst: &mut MaybeUninit<ValRaw>,
    ) -> Result<()> {
        assert!(Params::flatten_count() > MAX_STACK_PARAMS);

        // Memory must exist via validation if the arguments are stored on the
        // heap, so we can create a `MemoryMut` at this point. Afterwards
        // `realloc` is used to allocate space for all the arguments and then
        // they're all stored in linear memory.
        //
        // Note that `realloc` will bake in a check that the returned pointer is
        // in-bounds.
        let mut memory = MemoryMut::new(store.as_context_mut(), options);
        let ptr = memory.realloc(0, 0, Params::align(), Params::size())?;
        params.store(&mut memory, ptr)?;

        // Note that the pointer here is stored as a 64-bit integer. This allows
        // this to work with either 32 or 64-bit memories. For a 32-bit memory
        // it'll just ignore the upper 32 zero bits, and for 64-bit memories
        // this'll have the full 64-bits. Note that for 32-bit memories the call
        // to `realloc` above guarantees that the `ptr` is in-bounds meaning
        // that we will know that the zero-extended upper bits of `ptr` are
        // guaranteed to be zero.
        //
        // This comment about 64-bit integers is also referred to below with
        // "WRITEPTR64".
        dst.write(ValRaw::i64(ptr as i64));

        Ok(())
    }

    /// Lift the result of a function directly from the stack result.
    ///
    /// This is only used when the result fits in the maximum number of stack
    /// slots.
    fn lift_stack_result(
        store: &StoreOpaque,
        options: &Options,
        dst: &Return::Lower,
    ) -> Result<Return> {
        assert!(Return::flatten_count() <= MAX_STACK_RESULTS);
        Return::lift(store, options, dst)
    }

    /// Lift the result of a function where the result is stored indirectly on
    /// the heap.
    fn lift_heap_result(store: &StoreOpaque, options: &Options, dst: &ValRaw) -> Result<Return> {
        assert!(Return::flatten_count() > MAX_STACK_RESULTS);
        // FIXME: needs to read an i64 for memory64
        let ptr = usize::try_from(dst.get_u32())?;
        if ptr % usize::try_from(Return::align())? != 0 {
            bail!("return pointer not aligned");
        }

        let memory = Memory::new(store, options);
        let bytes = memory
            .as_slice()
            .get(ptr..)
            .and_then(|b| b.get(..Return::size()))
            .ok_or_else(|| anyhow::anyhow!("pointer out of bounds of memory"))?;
        Return::load(&memory, bytes)
    }

    /// Invokes the underlying wasm function, lowering arguments and lifting the
    /// result.
    ///
    /// The `lower` function and `lift` function provided here are what actually
    /// do the lowering and lifting. The `LowerParams` and `LowerReturn` types
    /// are what will be allocated on the stack for this function call. They
    /// should be appropriately sized for the lowering/lifting operation
    /// happening.
    fn call_raw<T, LowerParams, LowerReturn>(
        &self,
        store: &mut StoreContextMut<'_, T>,
        params: &Params,
        lower: impl FnOnce(
            &mut StoreContextMut<'_, T>,
            &Options,
            &Params,
            &mut MaybeUninit<LowerParams>,
        ) -> Result<()>,
        lift: impl FnOnce(&StoreOpaque, &Options, &LowerReturn) -> Result<Return>,
    ) -> Result<Return>
    where
        LowerParams: Copy,
        LowerReturn: Copy,
    {
        let super::FuncData {
            trampoline,
            export,
            options,
            instance,
            ..
        } = store.0[self.func.0];

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

        let instance = store.0[instance.0].as_ref().unwrap().instance();
        let flags = instance.flags();

        unsafe {
            if !(*flags).may_enter() {
                bail!("cannot reenter component instance");
            }
            debug_assert!((*flags).may_leave());

            (*flags).set_may_leave(false);
            let result = lower(store, &options, params, map_maybe_uninit!(space.params));
            (*flags).set_may_leave(true);
            result?;

            // This is unsafe as we are providing the guarantee that all the
            // inputs are valid. The various pointers passed in for the function
            // are all valid since they're coming from our store, and the
            // `params_and_results` should have the correct layout for the core
            // wasm function we're calling. Note that this latter point relies
            // on the correctness of this module and `ComponentValue`
            // implementations, hence `ComponentValue` being an `unsafe` trait.
            crate::Func::call_unchecked_raw(
                store,
                export.anyfunc,
                trampoline,
                space.as_mut_ptr().cast(),
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
            // Initially the `may_enter` flag is set to `false` for this
            // component instance and additionally we set a flag indicating that
            // a post-return is required. This isn't specified by the component
            // model itself but is used for our implementation of the API of
            // `post_return` as a separate function call.
            //
            // FIXME(WebAssembly/component-model#55) it's not really clear what
            // the semantics should be in the face of a lift error/trap. For now
            // the flags are reset so the instance can continue to be reused in
            // tests but that probably isn't what's desired.
            //
            // Otherwise though after a successful lift the return value of the
            // function, which is currently required to be 0 or 1 values
            // according to the canonical ABI, is saved within the `Store`'s
            // `FuncData`. This'll later get used in post-return.
            (*flags).set_may_enter(false);
            (*flags).set_needs_post_return(true);
            match lift(store.0, &options, ret) {
                Ok(val) => {
                    let ret_slice = cast_storage(ret);
                    let data = &mut store.0[self.func.0];
                    assert!(data.post_return_arg.is_none());
                    match ret_slice.len() {
                        0 => data.post_return_arg = Some(ValRaw::i32(0)),
                        1 => data.post_return_arg = Some(ret_slice[0]),
                        _ => unreachable!(),
                    }
                    return Ok(val);
                }
                Err(err) => {
                    (*flags).set_may_enter(true);
                    (*flags).set_needs_post_return(false);
                    return Err(err);
                }
            }
        }

        unsafe fn cast_storage<T>(storage: &T) -> &[ValRaw] {
            assert!(std::mem::size_of_val(storage) % std::mem::size_of::<ValRaw>() == 0);
            assert!(std::mem::align_of_val(storage) == std::mem::align_of::<ValRaw>());

            std::slice::from_raw_parts(
                (storage as *const T).cast(),
                mem::size_of_val(storage) / mem::size_of::<ValRaw>(),
            )
        }
    }

    /// Invokes the `post-return` canonical ABI option, if specified, after a
    /// [`TypedFunc::call`] has finished.
    ///
    /// For some more information on when to use this function see the
    /// documentation for post-return in the [`TypedFunc::call`] method.
    /// Otherwise though this function is a required method call after a
    /// [`TypedFunc::call`] completes successfully. After the embedder has
    /// finished processing the return value then this function must be invoked.
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
    /// [`TypedFunc::call`] completes successfully, and this function can only
    /// be called for the same [`TypedFunc`] that was `call`'d.
    ///
    /// If this function is called when [`TypedFunc::call`] was not previously
    /// called, then it will panic. If a different [`TypedFunc`] for the same
    /// component instance was invoked then this function will also panic
    /// because the `post-return` needs to happen for the other function.
    pub fn post_return(&self, mut store: impl AsContextMut) -> Result<()> {
        let mut store = store.as_context_mut();
        let data = &mut store.0[self.func.0];
        let instance = data.instance;
        let post_return = data.post_return;
        let post_return_arg = data.post_return_arg.take();
        let instance = store.0[instance.0].as_ref().unwrap().instance();
        let flags = instance.flags();

        unsafe {
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
                (*flags).needs_post_return(),
                "post_return can only be called after a function has previously been called",
            );
            let post_return_arg = post_return_arg.expect("calling post_return on wrong function");

            // This is a sanity-check assert which shouldn't ever trip.
            assert!(!(*flags).may_enter());

            // With the state of the world validated these flags are updated to
            // their component-model-defined states.
            (*flags).set_may_enter(true);
            (*flags).set_needs_post_return(false);

            // And finally if the function actually had a `post-return`
            // configured in its canonical options that's executed here.
            let (func, trampoline) = match post_return {
                Some(pair) => pair,
                None => return Ok(()),
            };
            crate::Func::call_unchecked_raw(
                &mut store,
                func.anyfunc,
                trampoline,
                &post_return_arg as *const ValRaw as *mut ValRaw,
            )?;
        }
        Ok(())
    }
}

#[repr(C)]
union ParamsAndResults<Params: Copy, Return: Copy> {
    params: Params,
    ret: Return,
}

/// A trait representing a static list of parameters that can be passed to a
/// [`TypedFunc`].
///
/// This trait is implemented for a number of tuple types and is not expected
/// to be implemented externally. The contents of this trait are hidden as it's
/// intended to be an implementation detail of Wasmtime. The contents of this
/// trait are not covered by Wasmtime's stability guarantees.
///
/// For more information about this trait see [`Func::typed`] and
/// [`TypedFunc`].
//
// Note that this is an `unsafe` trait, and the unsafety means that
// implementations of this trait must be correct or otherwise [`TypedFunc`]
// would not be memory safe. The main reason this is `unsafe` is the
// `typecheck` function which must operate correctly relative to the `AsTuple`
// interpretation of the implementor.
pub unsafe trait ComponentParams: ComponentType {
    /// Performs a typecheck to ensure that this `ComponentParams` implementor
    /// matches the types of the types in `params`.
    #[doc(hidden)]
    fn typecheck_params(
        params: &[(Option<String>, InterfaceType)],
        types: &ComponentTypes,
    ) -> Result<()>;
}

/// A trait representing types which can be passed to and read from components
/// with the canonical ABI.
///
/// This trait is implemented for Rust types which can be communicated to
/// components. This is implemented for Rust types which correspond to
/// interface types in the component model of WebAssembly. The [`Func::typed`]
/// and [`TypedFunc`] Rust items are the main consumers of this trait.
///
/// For more information on this trait see the examples in [`Func::typed`].
///
/// The contents of this trait are hidden as it's intended to be an
/// implementation detail of Wasmtime. The contents of this trait are not
/// covered by Wasmtime's stability guarantees.
//
// Note that this is an `unsafe` trait as `TypedFunc`'s safety heavily relies on
// the correctness of the implementations of this trait. Some ways in which this
// trait must be correct to be safe are:
//
// * The `Lower` associated type must be a `ValRaw` sequence. It doesn't have to
//   literally be `[ValRaw; N]` but when laid out in memory it must be adjacent
//   `ValRaw` values and have a multiple of the size of `ValRaw` and the same
//   alignment.
//
// * The `lower` function must initialize the bits within `Lower` that are going
//   to be read by the trampoline that's used to enter core wasm. A trampoline
//   is passed `*mut Lower` and will read the canonical abi arguments in
//   sequence, so all of the bits must be correctly initialized.
//
// * The `size` and `align` functions must be correct for this value stored in
//   the canonical ABI. The `Cursor<T>` iteration of these bytes rely on this
//   for correctness as they otherwise eschew bounds-checking.
//
// There are likely some other correctness issues which aren't documented as
// well, this isn't intended to be an exhaustive list. It suffices to say,
// though, that correctness bugs in this trait implementation are highly likely
// to lead to security bugs, which again leads to the `unsafe` in the trait.
//
// Also note that this trait specifically is not sealed because we'll
// eventually have a proc macro that generates implementations of this trait
// for external types in a `#[derive]`-like fashion.
//
// FIXME: need to write a #[derive(ComponentType)]
pub unsafe trait ComponentType {
    /// Representation of the "lowered" form of this component value.
    ///
    /// Lowerings lower into core wasm values which are represented by `ValRaw`.
    /// This `Lower` type must be a list of `ValRaw` as either a literal array
    /// or a struct where every field is a `ValRaw`. This must be `Copy` (as
    /// `ValRaw` is `Copy`) and support all byte patterns. This being correct is
    /// one reason why the trait is unsafe.
    #[doc(hidden)]
    type Lower: Copy;

    /// Returns the number of core wasm abi values will be used to represent
    /// this type in its lowered form.
    ///
    /// This divides the size of `Self::Lower` by the size of `ValRaw`.
    #[doc(hidden)]
    fn flatten_count() -> usize {
        assert!(mem::size_of::<Self::Lower>() % mem::size_of::<ValRaw>() == 0);
        assert!(mem::align_of::<Self::Lower>() == mem::align_of::<ValRaw>());
        mem::size_of::<Self::Lower>() / mem::size_of::<ValRaw>()
    }

    /// Returns the size, in bytes, that this type has in the canonical ABI.
    ///
    /// Note that it's expected that this function is "simple" to be easily
    /// optimizable by LLVM (e.g. inlined and const-evaluated).
    //
    // FIXME: needs some sort of parameter indicating the memory size
    #[doc(hidden)]
    fn size() -> usize;

    /// Returns the alignment, in bytes, that this type has in the canonical
    /// ABI.
    ///
    /// Note that it's expected that this function is "simple" to be easily
    /// optimizable by LLVM (e.g. inlined and const-evaluated).
    #[doc(hidden)]
    fn align() -> u32;

    /// Performs a type-check to see whether this comopnent value type matches
    /// the interface type `ty` provided.
    ///
    /// The `op` provided is the operations which could be performed with this
    /// type if the typecheck passes, either lifting or lowering. Some Rust
    /// types are only valid for one operation and we can't prevent the wrong
    /// one from being used at compile time so we rely on the runtime check
    /// here.
    #[doc(hidden)]
    fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()>;
}

/// Host types which can be passed to WebAssembly components.
///
/// This trait is implemented for all types that can be passed to components
/// either as parameters of component exports or returns of component imports.
/// This trait represents the ability to convert from the native host
/// representation to the canonical ABI.
//
// TODO: #[derive(Lower)]
// TODO: more docs here
pub unsafe trait Lower: ComponentType {
    /// Performs the "lower" function in the canonical ABI.
    ///
    /// This method will lower the given value into wasm linear memory. The
    /// `store` and `func` are provided in case memory is needed (e.g. for
    /// strings/lists) so `realloc` can be called. The `dst` is the destination
    /// to store the lowered results.
    ///
    /// Note that `dst` is a pointer to uninitialized memory. It's expected
    /// that `dst` is fully initialized by the time this function returns, hence
    /// the `unsafe` on the trait implementation.
    ///
    /// This will only be called if `typecheck` passes for `Op::Lower`.
    #[doc(hidden)]
    fn lower<T>(
        &self,
        store: &mut StoreContextMut<T>,
        options: &Options,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()>;

    /// Performs the "store" operation in the canonical ABI.
    ///
    /// This function will store `self` into the linear memory described by
    /// `memory` at the `offset` provided.
    ///
    /// It is expected that `offset` is a valid offset in memory for
    /// `Self::size()` bytes. At this time that's not an unsafe contract as it's
    /// always re-checked on all stores, but this is something that will need to
    /// be improved in the future to remove extra bounds checks. For now this
    /// function will panic if there's a bug and `offset` isn't valid within
    /// memory.
    ///
    /// This will only be called if `typecheck` passes for `Op::Lower`.
    #[doc(hidden)]
    fn store<T>(&self, memory: &mut MemoryMut<'_, T>, offset: usize) -> Result<()>;
}

/// Host types which can be created from the canonical ABI.
//
// TODO: #[derive(Lower)]
// TODO: more docs here
pub unsafe trait Lift: Sized + ComponentType {
    /// Performs the "lift" operation in the canonical ABI.
    ///
    /// This will read the core wasm values from `src` and use the memory
    /// specified by `func` and `store` optionally if necessary. An instance of
    /// `Self` is then created from the values, assuming validation succeeds.
    ///
    /// Note that this has a default implementation but if `typecheck` passes
    /// for `Op::Lift` this needs to be overridden.
    #[doc(hidden)]
    fn lift(store: &StoreOpaque, options: &Options, src: &Self::Lower) -> Result<Self>;

    /// Performs the "load" operation in the canonical ABI.
    ///
    /// This is given the linear-memory representation of `Self` in the `bytes`
    /// array provided which is guaranteed to be `Self::size()` bytes large. All
    /// of memory is then also described with `Memory` for bounds-checks and
    /// such as necessary for strings/lists.
    ///
    /// Note that this has a default implementation but if `typecheck` passes
    /// for `Op::Lift` this needs to be overridden.
    #[doc(hidden)]
    fn load(memory: &Memory<'_>, bytes: &[u8]) -> Result<Self>;
}

// Macro to help generate "forwarding implementations" of `ComponentType` to
// another type, used for wrappers in Rust like `&T`, `Box<T>`, etc. Note that
// these wrappers only implement lowering because lifting native Rust types
// cannot be done.
macro_rules! forward_impls {
    ($(($($generics:tt)*) $a:ty => $b:ty,)*) => ($(
        unsafe impl <$($generics)*> ComponentType for $a {
            type Lower = <$b as ComponentType>::Lower;

            #[inline]
            fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()> {
                <$b as ComponentType>::typecheck(ty, types)
            }

            #[inline]
            fn size() -> usize {
                <$b as ComponentType>::size()
            }

            #[inline]
            fn align() -> u32 {
                <$b as ComponentType>::align()
            }
        }

        unsafe impl <$($generics)*> Lower for $a {
            fn lower<U>(
                &self,
                store: &mut StoreContextMut<U>,
                options: &Options,
                dst: &mut MaybeUninit<Self::Lower>,
            ) -> Result<()> {
                <$b as Lower>::lower(self, store, options, dst)
            }

            fn store<U>(&self, memory: &mut MemoryMut<'_, U>, offset: usize) -> Result<()> {
                <$b as Lower>::store(self, memory, offset)
            }
        }
    )*)
}

forward_impls! {
    (T: Lower + ?Sized) &'_ T => T,
    (T: Lower + ?Sized) Box<T> => T,
    (T: Lower + ?Sized) std::rc::Rc<T> => T,
    (T: Lower + ?Sized) std::sync::Arc<T> => T,
    () String => str,
    (T: Lower) Vec<T> => [T],
}

// Macro to help generate `ComponentValue` implementations for primitive types
// such as integers, char, bool, etc.
macro_rules! integers {
    ($($primitive:ident = $ty:ident in $field:ident/$get:ident,)*) => ($(
        unsafe impl ComponentType for $primitive {
            type Lower = ValRaw;

            fn typecheck(ty: &InterfaceType, _types: &ComponentTypes) -> Result<()> {
                match ty {
                    InterfaceType::$ty => Ok(()),
                    other => bail!("expected `{}` found `{}`", desc(&InterfaceType::$ty), desc(other))
                }
            }

            #[inline]
            fn size() -> usize { mem::size_of::<$primitive>() }

            // Note that this specifically doesn't use `align_of` as some
            // host platforms have a 4-byte alignment for primitive types but
            // the canonical abi always has the same size/alignment for these
            // types.
            #[inline]
            fn align() -> u32 { mem::size_of::<$primitive>() as u32 }
        }

        unsafe impl Lower for $primitive {
            fn lower<T>(
                &self,
                _store: &mut StoreContextMut<T>,
                _options: &Options,
                dst: &mut MaybeUninit<Self::Lower>,
            ) -> Result<()> {
                dst.write(ValRaw::$field(*self as $field));
                Ok(())
            }

            fn store<T>(&self, memory: &mut MemoryMut<'_, T>, offset: usize) -> Result<()> {
                debug_assert!(offset % Self::size() == 0);
                *memory.get(offset) = self.to_le_bytes();
                Ok(())
            }
        }

        unsafe impl Lift for $primitive {
            #[inline]
            fn lift(_store: &StoreOpaque, _options: &Options, src: &Self::Lower) -> Result<Self> {
                Ok(src.$get() as $primitive)
            }

            #[inline]
            fn load(_mem: &Memory<'_>, bytes: &[u8]) -> Result<Self> {
                debug_assert!((bytes.as_ptr() as usize) % Self::size() == 0);
                Ok($primitive::from_le_bytes(bytes.try_into().unwrap()))
            }
        }
    )*)
}

integers! {
    i8 = S8 in i32/get_i32,
    u8 = U8 in u32/get_u32,
    i16 = S16 in i32/get_i32,
    u16 = U16 in u32/get_u32,
    i32 = S32 in i32/get_i32,
    u32 = U32 in u32/get_u32,
    i64 = S64 in i64/get_i64,
    u64 = U64 in u64/get_u64,
}

macro_rules! floats {
    ($($float:ident/$get_float:ident = $ty:ident)*) => ($(const _: () = {
        /// All floats in-and-out of the canonical abi always have their nan
        /// payloads canonicalized. conveniently the `NAN` constant in rust has
        /// the same representation as canonical nan, so we can use that for the
        /// nan value.
        #[inline]
        fn canonicalize(float: $float) -> $float {
            if float.is_nan() {
                $float::NAN
            } else {
                float
            }
        }

        unsafe impl ComponentType for $float {
            type Lower = ValRaw;

            fn typecheck(ty: &InterfaceType, _types: &ComponentTypes) -> Result<()> {
                match ty {
                    InterfaceType::$ty => Ok(()),
                    other => bail!("expected `{}` found `{}`", desc(&InterfaceType::$ty), desc(other))
                }
            }

            #[inline]
            fn size() -> usize { mem::size_of::<$float>() }

            // note that like integers size is used here instead of alignment to
            // respect the canonical abi, not host platforms.
            #[inline]
            fn align() -> u32 { mem::size_of::<$float>() as u32 }
        }

        unsafe impl Lower for $float {
            fn lower<T>(
                &self,
                _store: &mut StoreContextMut<T>,
                _options: &Options,
                dst: &mut MaybeUninit<Self::Lower>,
            ) -> Result<()> {
                dst.write(ValRaw::$float(canonicalize(*self).to_bits()));
                Ok(())
            }

            fn store<T>(&self, memory: &mut MemoryMut<'_, T>, offset: usize) -> Result<()> {
                debug_assert!(offset % Self::size() == 0);
                let ptr = memory.get(offset);
                *ptr = canonicalize(*self).to_bits().to_le_bytes();
                Ok(())
            }
        }

        unsafe impl Lift for $float {
            #[inline]
            fn lift(_store: &StoreOpaque, _options: &Options, src: &Self::Lower) -> Result<Self> {
                Ok(canonicalize($float::from_bits(src.$get_float())))
            }

            #[inline]
            fn load(_mem: &Memory<'_>, bytes: &[u8]) -> Result<Self> {
                debug_assert!((bytes.as_ptr() as usize) % Self::size() == 0);
                Ok(canonicalize($float::from_le_bytes(bytes.try_into().unwrap())))
            }
        }
    };)*)
}

floats! {
    f32/get_f32 = Float32
    f64/get_f64 = Float64
}

unsafe impl ComponentType for bool {
    type Lower = ValRaw;

    fn typecheck(ty: &InterfaceType, _types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::Bool => Ok(()),
            other => bail!("expected `bool` found `{}`", desc(other)),
        }
    }

    #[inline]
    fn size() -> usize {
        1
    }

    #[inline]
    fn align() -> u32 {
        1
    }
}

unsafe impl Lower for bool {
    fn lower<T>(
        &self,
        _store: &mut StoreContextMut<T>,
        _options: &Options,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        dst.write(ValRaw::i32(*self as i32));
        Ok(())
    }

    fn store<T>(&self, memory: &mut MemoryMut<'_, T>, offset: usize) -> Result<()> {
        debug_assert!(offset % Self::size() == 0);
        memory.get::<1>(offset)[0] = *self as u8;
        Ok(())
    }
}

unsafe impl Lift for bool {
    #[inline]
    fn lift(_store: &StoreOpaque, _options: &Options, src: &Self::Lower) -> Result<Self> {
        match src.get_i32() {
            0 => Ok(false),
            _ => Ok(true),
        }
    }

    #[inline]
    fn load(_mem: &Memory<'_>, bytes: &[u8]) -> Result<Self> {
        match bytes[0] {
            0 => Ok(false),
            _ => Ok(true),
        }
    }
}

unsafe impl ComponentType for char {
    type Lower = ValRaw;

    fn typecheck(ty: &InterfaceType, _types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::Char => Ok(()),
            other => bail!("expected `char` found `{}`", desc(other)),
        }
    }

    #[inline]
    fn size() -> usize {
        4
    }

    #[inline]
    fn align() -> u32 {
        4
    }
}

unsafe impl Lower for char {
    fn lower<T>(
        &self,
        _store: &mut StoreContextMut<T>,
        _options: &Options,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        dst.write(ValRaw::u32(u32::from(*self)));
        Ok(())
    }

    fn store<T>(&self, memory: &mut MemoryMut<'_, T>, offset: usize) -> Result<()> {
        debug_assert!(offset % Self::size() == 0);
        *memory.get::<4>(offset) = u32::from(*self).to_le_bytes();
        Ok(())
    }
}

unsafe impl Lift for char {
    #[inline]
    fn lift(_store: &StoreOpaque, _options: &Options, src: &Self::Lower) -> Result<Self> {
        Ok(char::try_from(src.get_u32())?)
    }

    #[inline]
    fn load(_memory: &Memory<'_>, bytes: &[u8]) -> Result<Self> {
        debug_assert!((bytes.as_ptr() as usize) % Self::size() == 0);
        let bits = u32::from_le_bytes(bytes.try_into().unwrap());
        Ok(char::try_from(bits)?)
    }
}

// Note that this is similar to `ComponentValue for WasmStr` except it can only
// be used for lowering, not lifting.
unsafe impl ComponentType for str {
    type Lower = [ValRaw; 2];

    fn typecheck(ty: &InterfaceType, _types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::String => Ok(()),
            other => bail!("expected `string` found `{}`", desc(other)),
        }
    }
    fn size() -> usize {
        8
    }

    fn align() -> u32 {
        4
    }
}

unsafe impl Lower for str {
    fn lower<T>(
        &self,
        store: &mut StoreContextMut<T>,
        options: &Options,
        dst: &mut MaybeUninit<[ValRaw; 2]>,
    ) -> Result<()> {
        let (ptr, len) = lower_string(&mut MemoryMut::new(store.as_context_mut(), options), self)?;
        // See "WRITEPTR64" above for why this is always storing a 64-bit
        // integer.
        map_maybe_uninit!(dst[0]).write(ValRaw::i64(ptr as i64));
        map_maybe_uninit!(dst[1]).write(ValRaw::i64(len as i64));
        Ok(())
    }

    fn store<T>(&self, mem: &mut MemoryMut<'_, T>, offset: usize) -> Result<()> {
        debug_assert!(offset % (Self::align() as usize) == 0);
        let (ptr, len) = lower_string(mem, self)?;
        // FIXME: needs memory64 handling
        *mem.get(offset + 0) = (ptr as i32).to_le_bytes();
        *mem.get(offset + 4) = (len as i32).to_le_bytes();
        Ok(())
    }
}

fn lower_string<T>(mem: &mut MemoryMut<'_, T>, string: &str) -> Result<(usize, usize)> {
    match mem.string_encoding() {
        StringEncoding::Utf8 => {
            let ptr = mem.realloc(0, 0, 1, string.len())?;
            mem.as_slice_mut()[ptr..][..string.len()].copy_from_slice(string.as_bytes());
            Ok((ptr, string.len()))
        }
        StringEncoding::Utf16 => {
            let size = string.len() * 2;
            let mut ptr = mem.realloc(0, 0, 2, size)?;
            let bytes = &mut mem.as_slice_mut()[ptr..][..size];
            let mut copied = 0;
            for (u, bytes) in string.encode_utf16().zip(bytes.chunks_mut(2)) {
                let u_bytes = u.to_le_bytes();
                bytes[0] = u_bytes[0];
                bytes[1] = u_bytes[1];
                copied += 1;
            }
            if (copied * 2) < size {
                ptr = mem.realloc(ptr, size, 2, copied * 2)?;
            }
            Ok((ptr, copied))
        }
        StringEncoding::CompactUtf16 => {
            unimplemented!("compact-utf-16");
        }
    }
}

/// Representation of a string located in linear memory in a WebAssembly
/// instance.
///
/// This type is used with [`TypedFunc`], for example, when WebAssembly returns
/// a string. This type cannot be used to give a string to WebAssembly, instead
/// `&str` should be used for that (since it's coming from the host).
///
/// Note that this type represents an in-bounds string in linear memory, but it
/// does not represent a valid string (e.g. valid utf-8). Validation happens
/// when [`WasmStr::to_str`] is called.
//
// TODO: should probably expand this with examples
pub struct WasmStr {
    ptr: usize,
    len: usize,
    options: Options,
}

impl WasmStr {
    fn new(ptr: usize, len: usize, memory: &Memory<'_>) -> Result<WasmStr> {
        let byte_len = match memory.string_encoding() {
            StringEncoding::Utf8 => Some(len),
            StringEncoding::Utf16 => len.checked_mul(2),
            StringEncoding::CompactUtf16 => unimplemented!(),
        };
        match byte_len.and_then(|len| ptr.checked_add(len)) {
            Some(n) if n <= memory.as_slice().len() => {}
            _ => bail!("string pointer/length out of bounds of memory"),
        }
        Ok(WasmStr {
            ptr,
            len,
            options: *memory.options(),
        })
    }

    /// Returns the underlying string that this cursor points to.
    ///
    /// Note that this will internally decode the string from the wasm's
    /// encoding to utf-8 and additionally perform validation.
    ///
    /// The `store` provided must be the store where this string lives to
    /// access the correct memory.
    ///
    /// # Errors
    ///
    /// Returns an error if the string wasn't encoded correctly (e.g. invalid
    /// utf-8).
    ///
    /// # Panics
    ///
    /// Panics if this string is not owned by `store`.
    //
    // TODO: should add accessors for specifically utf-8 and utf-16 that perhaps
    // in an opt-in basis don't do validation. Additionally there should be some
    // method that returns `[u16]` after validating to avoid the utf16-to-utf8
    // transcode.
    pub fn to_str<'a, T: 'a>(&self, store: impl Into<StoreContext<'a, T>>) -> Result<Cow<'a, str>> {
        self._to_str(store.into().0)
    }

    fn _to_str<'a>(&self, store: &'a StoreOpaque) -> Result<Cow<'a, str>> {
        match self.options.string_encoding() {
            StringEncoding::Utf8 => self.decode_utf8(store),
            StringEncoding::Utf16 => self.decode_utf16(store),
            StringEncoding::CompactUtf16 => unimplemented!(),
        }
    }

    fn decode_utf8<'a>(&self, store: &'a StoreOpaque) -> Result<Cow<'a, str>> {
        let memory = self.options.memory(store);
        // Note that bounds-checking already happen in construction of `WasmStr`
        // so this is never expected to panic. This could theoretically be
        // unchecked indexing if we're feeling wild enough.
        Ok(str::from_utf8(&memory[self.ptr..][..self.len])?.into())
    }

    fn decode_utf16<'a>(&self, store: &'a StoreOpaque) -> Result<Cow<'a, str>> {
        let memory = self.options.memory(store);
        // See notes in `decode_utf8` for why this is panicking indexing.
        let memory = &memory[self.ptr..][..self.len * 2];
        Ok(std::char::decode_utf16(
            memory
                .chunks(2)
                .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap())),
        )
        .collect::<Result<String, _>>()?
        .into())
    }
}

// Note that this is similar to `ComponentValue for str` except it can only be
// used for lifting, not lowering.
unsafe impl ComponentType for WasmStr {
    type Lower = <str as ComponentType>::Lower;

    #[inline]
    fn size() -> usize {
        <str as ComponentType>::size()
    }

    #[inline]
    fn align() -> u32 {
        <str as ComponentType>::align()
    }

    fn typecheck(ty: &InterfaceType, _types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::String => Ok(()),
            other => bail!("expected `string` found `{}`", desc(other)),
        }
    }
}

unsafe impl Lift for WasmStr {
    fn lift(store: &StoreOpaque, options: &Options, src: &Self::Lower) -> Result<Self> {
        // FIXME: needs memory64 treatment
        let ptr = src[0].get_u32();
        let len = src[1].get_u32();
        let (ptr, len) = (usize::try_from(ptr)?, usize::try_from(len)?);
        WasmStr::new(ptr, len, &Memory::new(store, options))
    }

    fn load(memory: &Memory<'_>, bytes: &[u8]) -> Result<Self> {
        debug_assert!((bytes.as_ptr() as usize) % (Self::align() as usize) == 0);
        // FIXME: needs memory64 treatment
        let ptr = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        let len = u32::from_le_bytes(bytes[4..].try_into().unwrap());
        let (ptr, len) = (usize::try_from(ptr)?, usize::try_from(len)?);
        WasmStr::new(ptr, len, memory)
    }
}

unsafe impl<T> ComponentType for [T]
where
    T: ComponentType,
{
    type Lower = [ValRaw; 2];

    fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::List(t) => T::typecheck(&types[*t], types),
            other => bail!("expected `list` found `{}`", desc(other)),
        }
    }

    #[inline]
    fn size() -> usize {
        8
    }

    #[inline]
    fn align() -> u32 {
        4
    }
}

unsafe impl<T> Lower for [T]
where
    T: Lower,
{
    fn lower<U>(
        &self,
        store: &mut StoreContextMut<U>,
        options: &Options,
        dst: &mut MaybeUninit<[ValRaw; 2]>,
    ) -> Result<()> {
        let (ptr, len) = lower_list(&mut MemoryMut::new(store.as_context_mut(), options), self)?;
        // See "WRITEPTR64" above for why this is always storing a 64-bit
        // integer.
        map_maybe_uninit!(dst[0]).write(ValRaw::i64(ptr as i64));
        map_maybe_uninit!(dst[1]).write(ValRaw::i64(len as i64));
        Ok(())
    }

    fn store<U>(&self, mem: &mut MemoryMut<'_, U>, offset: usize) -> Result<()> {
        debug_assert!(offset % (Self::align() as usize) == 0);
        let (ptr, len) = lower_list(mem, self)?;
        *mem.get(offset + 0) = (ptr as i32).to_le_bytes();
        *mem.get(offset + 4) = (len as i32).to_le_bytes();
        Ok(())
    }
}

// FIXME: this is not a memcpy for `T` where `T` is something like `u8`.
//
// Some attempts to fix this have proved not fruitful. In isolation an attempt
// was made where:
//
// * `MemoryMut` stored a `*mut [u8]` as its "last view" of memory to avoid
//   reloading the base pointer constantly. This view is reset on `realloc`.
// * The bounds-checks in `MemoryMut::get` were removed (replaced with unsafe
//   indexing)
//
// Even then though this didn't correctly vectorized for `Vec<u8>`. It's not
// entirely clear why but it appeared that it's related to reloading the base
// pointer fo memory (I guess from `MemoryMut` itself?). Overall I'm not really
// clear on what's happening there, but this is surely going to be a performance
// bottleneck in the future.
fn lower_list<T, U>(mem: &mut MemoryMut<'_, U>, list: &[T]) -> Result<(usize, usize)>
where
    T: Lower,
{
    let elem_size = T::size();
    let size = list
        .len()
        .checked_mul(elem_size)
        .ok_or_else(|| anyhow::anyhow!("size overflow copying a list"))?;
    let ptr = mem.realloc(0, 0, T::align(), size)?;
    let mut cur = ptr;
    for item in list {
        item.store(mem, cur)?;
        cur += elem_size;
    }
    Ok((ptr, list.len()))
}

/// Representation of a list of values that are owned by a WebAssembly instance.
///
/// This type is used whenever a `(list T)` is returned from a [`TypedFunc`],
/// for example. This type represents a list of values that are stored in linear
/// memory which are waiting to be read.
///
/// Note that this type represents only a valid range of bytes for the list
/// itself, it does not represent validity of the elements themselves and that's
/// performed when they're iterated.
pub struct WasmList<T> {
    ptr: usize,
    len: usize,
    options: Options,
    _marker: marker::PhantomData<T>,
}

impl<T: Lift> WasmList<T> {
    fn new(ptr: usize, len: usize, memory: &Memory<'_>) -> Result<WasmList<T>> {
        match len
            .checked_mul(T::size())
            .and_then(|len| ptr.checked_add(len))
        {
            Some(n) if n <= memory.as_slice().len() => {}
            _ => bail!("list pointer/length out of bounds of memory"),
        }
        if ptr % usize::try_from(T::align())? != 0 {
            bail!("list pointer is not aligned")
        }
        Ok(WasmList {
            ptr,
            len,
            options: *memory.options(),
            _marker: marker::PhantomData,
        })
    }

    /// Returns the item length of this vector
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Gets the `n`th element of this list.
    ///
    /// Returns `None` if `index` is out of bounds. Returns `Some(Err(..))` if
    /// the value couldn't be decoded (it was invalid). Returns `Some(Ok(..))`
    /// if the value is valid.
    //
    // TODO: given that interface values are intended to be consumed in one go
    // should we even expose a random access iteration API? In theory all
    // consumers should be validating through the iterator.
    pub fn get(&self, store: impl AsContext, index: usize) -> Option<Result<T>> {
        self._get(store.as_context().0, index)
    }

    fn _get(&self, store: &StoreOpaque, index: usize) -> Option<Result<T>> {
        if index >= self.len {
            return None;
        }
        let memory = Memory::new(store, &self.options);
        // Note that this is using panicking indexing and this is expected to
        // never fail. The bounds-checking here happened during the construction
        // of the `WasmList` itself which means these should always be in-bounds
        // (and wasm memory can only grow). This could theoretically be
        // unchecked indexing if we're confident enough and it's actually a perf
        // issue one day.
        let bytes = &memory.as_slice()[self.ptr + index * T::size()..][..T::size()];
        Some(T::load(&memory, bytes))
    }

    /// Returns an iterator over the elements of this list.
    ///
    /// Each item of the list may fail to decode and is represented through the
    /// `Result` value of the iterator.
    pub fn iter<'a, U: 'a>(
        &'a self,
        store: impl Into<StoreContext<'a, U>>,
    ) -> impl ExactSizeIterator<Item = Result<T>> + 'a {
        let store = store.into().0;
        (0..self.len).map(move |i| self._get(store, i).unwrap())
    }
}

impl WasmList<u8> {
    /// Get access to the raw underlying memory for this byte slice.
    ///
    /// Note that this is specifically only implemented for a `(list u8)` type
    /// since it's known to be valid in terms of alignment and representation
    /// validity.
    pub fn as_slice<'a, T: 'a>(&self, store: impl Into<StoreContext<'a, T>>) -> &'a [u8] {
        // See comments in `WasmList::get` for the panicking indexing
        &self.options.memory(store.into().0)[self.ptr..][..self.len]
    }
}

// Note that this is similar to `ComponentValue for str` except it can only be
// used for lifting, not lowering.
unsafe impl<T: ComponentType> ComponentType for WasmList<T> {
    type Lower = <[T] as ComponentType>::Lower;

    #[inline]
    fn size() -> usize {
        <[T] as ComponentType>::size()
    }

    fn align() -> u32 {
        <[T] as ComponentType>::align()
    }

    fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::List(t) => T::typecheck(&types[*t], types),
            other => bail!("expected `list` found `{}`", desc(other)),
        }
    }
}

unsafe impl<T: Lift> Lift for WasmList<T> {
    fn lift(store: &StoreOpaque, options: &Options, src: &Self::Lower) -> Result<Self> {
        // FIXME: needs memory64 treatment
        let ptr = src[0].get_u32();
        let len = src[1].get_u32();
        let (ptr, len) = (usize::try_from(ptr)?, usize::try_from(len)?);
        WasmList::new(ptr, len, &Memory::new(store, options))
    }

    fn load(memory: &Memory<'_>, bytes: &[u8]) -> Result<Self> {
        debug_assert!((bytes.as_ptr() as usize) % (Self::align() as usize) == 0);
        // FIXME: needs memory64 treatment
        let ptr = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        let len = u32::from_le_bytes(bytes[4..].try_into().unwrap());
        let (ptr, len) = (usize::try_from(ptr)?, usize::try_from(len)?);
        WasmList::new(ptr, len, memory)
    }
}

/// Round `a` up to the next multiple of `align`, assuming that `align` is a power of 2.
#[inline]
const fn align_to(a: usize, align: u32) -> usize {
    debug_assert!(align.is_power_of_two());
    let align = align as usize;
    (a + (align - 1)) & !(align - 1)
}

/// For a field of type T starting after `offset` bytes, updates the offset to reflect the correct
/// alignment and size of T. Returns the correctly aligned offset for the start of the field.
#[inline]
pub fn next_field<T: ComponentType>(offset: &mut usize) -> usize {
    *offset = align_to(*offset, T::align());
    let result = *offset;
    *offset += T::size();
    result
}

/// Verify that the given wasm type is a tuple with the expected fields in the right order.
#[inline]
fn typecheck_tuple(
    ty: &InterfaceType,
    types: &ComponentTypes,
    expected: &[fn(&InterfaceType, &ComponentTypes) -> Result<()>],
) -> Result<()> {
    match ty {
        InterfaceType::Unit if expected.len() == 0 => Ok(()),
        InterfaceType::Tuple(t) => {
            let tuple = &types[*t];
            if tuple.types.len() != expected.len() {
                if expected.len() == 0 {
                    bail!(
                        "expected unit or 0-tuple, found {}-tuple",
                        tuple.types.len(),
                    );
                }
                bail!(
                    "expected {}-tuple, found {}-tuple",
                    expected.len(),
                    tuple.types.len()
                );
            }
            for (ty, check) in tuple.types.iter().zip(expected) {
                check(ty, types)?;
            }
            Ok(())
        }
        other if expected.len() == 0 => {
            bail!("expected `unit` or 0-tuple found `{}`", desc(other))
        }
        other => bail!("expected `tuple` found `{}`", desc(other)),
    }
}

unsafe impl<T> ComponentType for Option<T>
where
    T: ComponentType,
{
    type Lower = TupleLower2<<u32 as ComponentType>::Lower, T::Lower>;

    fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::Option(t) => T::typecheck(&types[*t], types),
            other => bail!("expected `option` found `{}`", desc(other)),
        }
    }

    #[inline]
    fn size() -> usize {
        align_to(1, T::align()) + T::size()
    }

    #[inline]
    fn align() -> u32 {
        T::align()
    }
}

unsafe impl<T> Lower for Option<T>
where
    T: Lower,
{
    fn lower<U>(
        &self,
        store: &mut StoreContextMut<U>,
        options: &Options,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        match self {
            None => {
                map_maybe_uninit!(dst.A1).write(ValRaw::i32(0));
                // Note that this is unsafe as we're writing an arbitrary
                // bit-pattern to an arbitrary type, but part of the unsafe
                // contract of the `ComponentValue` trait is that we can assign
                // any bit-pattern. By writing all zeros here we're ensuring
                // that the core wasm arguments this translates to will all be
                // zeros (as the canonical ABI requires).
                unsafe {
                    map_maybe_uninit!(dst.A2).as_mut_ptr().write_bytes(0u8, 1);
                }
            }
            Some(val) => {
                map_maybe_uninit!(dst.A1).write(ValRaw::i32(1));
                val.lower(store, options, map_maybe_uninit!(dst.A2))?;
            }
        }
        Ok(())
    }

    fn store<U>(&self, mem: &mut MemoryMut<'_, U>, offset: usize) -> Result<()> {
        debug_assert!(offset % (Self::align() as usize) == 0);
        match self {
            None => {
                mem.get::<1>(offset)[0] = 0;
            }
            Some(val) => {
                mem.get::<1>(offset)[0] = 1;
                val.store(mem, offset + align_to(1, T::align()))?;
            }
        }
        Ok(())
    }
}

unsafe impl<T> Lift for Option<T>
where
    T: Lift,
{
    fn lift(store: &StoreOpaque, options: &Options, src: &Self::Lower) -> Result<Self> {
        Ok(match src.A1.get_i32() {
            0 => None,
            1 => Some(T::lift(store, options, &src.A2)?),
            _ => bail!("invalid option discriminant"),
        })
    }

    fn load(memory: &Memory<'_>, bytes: &[u8]) -> Result<Self> {
        debug_assert!((bytes.as_ptr() as usize) % (Self::align() as usize) == 0);
        let discrim = bytes[0];
        let payload = &bytes[align_to(1, T::align())..];
        match discrim {
            0 => Ok(None),
            1 => Ok(Some(T::load(memory, payload)?)),
            _ => bail!("invalid option discriminant"),
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ResultLower<T: Copy, E: Copy> {
    tag: ValRaw,
    payload: ResultLowerPayload<T, E>,
}

#[derive(Clone, Copy)]
#[repr(C)]
union ResultLowerPayload<T: Copy, E: Copy> {
    ok: T,
    err: E,
}

unsafe impl<T, E> ComponentType for Result<T, E>
where
    T: ComponentType,
    E: ComponentType,
{
    type Lower = ResultLower<T::Lower, E::Lower>;

    fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::Expected(r) => {
                let expected = &types[*r];
                T::typecheck(&expected.ok, types)?;
                E::typecheck(&expected.err, types)?;
                Ok(())
            }
            other => bail!("expected `expected` found `{}`", desc(other)),
        }
    }

    #[inline]
    fn size() -> usize {
        align_to(1, Self::align()) + T::size().max(E::size())
    }

    #[inline]
    fn align() -> u32 {
        T::align().max(E::align())
    }
}

unsafe impl<T, E> Lower for Result<T, E>
where
    T: Lower,
    E: Lower,
{
    fn lower<U>(
        &self,
        store: &mut StoreContextMut<U>,
        options: &Options,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        // Start out by zeroing out the payload. This will ensure that if either
        // arm doesn't initialize some values then everything is still
        // deterministically set.
        //
        // Additionally, this initialization of zero means that the specific
        // types written by each `lower` call below on each arm still has the
        // correct value even when "joined" with the other arm.
        //
        // Finally note that this is required by the canonical ABI to some
        // degree where if the `Ok` arm initializes fewer values than the `Err`
        // arm then all the remaining values must be initialized to zero, and
        // that's what this does.
        unsafe {
            map_maybe_uninit!(dst.payload)
                .as_mut_ptr()
                .write_bytes(0u8, 1);
        }

        match self {
            Ok(e) => {
                map_maybe_uninit!(dst.tag).write(ValRaw::i32(0));
                e.lower(store, options, map_maybe_uninit!(dst.payload.ok))?;
            }
            Err(e) => {
                map_maybe_uninit!(dst.tag).write(ValRaw::i32(1));
                e.lower(store, options, map_maybe_uninit!(dst.payload.err))?;
            }
        }
        Ok(())
    }

    fn store<U>(&self, mem: &mut MemoryMut<'_, U>, offset: usize) -> Result<()> {
        debug_assert!(offset % (Self::align() as usize) == 0);
        match self {
            Ok(e) => {
                mem.get::<1>(offset)[0] = 0;
                e.store(mem, offset + align_to(1, Self::align()))?;
            }
            Err(e) => {
                mem.get::<1>(offset)[0] = 1;
                e.store(mem, offset + align_to(1, Self::align()))?;
            }
        }
        Ok(())
    }
}

unsafe impl<T, E> Lift for Result<T, E>
where
    T: Lift,
    E: Lift,
{
    fn lift(store: &StoreOpaque, options: &Options, src: &Self::Lower) -> Result<Self> {
        // Note that this implementation specifically isn't trying to actually
        // reinterpret or alter the bits of `lower` depending on which variant
        // we're lifting. This ends up all working out because the value is
        // stored in little-endian format.
        //
        // When stored in little-endian format the `{T,E}::Lower`, when each
        // individual `ValRaw` is read, means that if an i64 value, extended
        // from an i32 value, was stored then when the i32 value is read it'll
        // automatically ignore the upper bits.
        //
        // This "trick" allows us to seamlessly pass through the `Self::Lower`
        // representation into the lifting/lowering without trying to handle
        // "join"ed types as per the canonical ABI. It just so happens that i64
        // bits will naturally be reinterpreted as f64. Additionally if the
        // joined type is i64 but only the lower bits are read that's ok and we
        // don't need to validate the upper bits.
        //
        // This is largely enabled by WebAssembly/component-model#35 where no
        // validation needs to be performed for ignored bits and bytes here.
        Ok(match src.tag.get_i32() {
            0 => Ok(unsafe { T::lift(store, options, &src.payload.ok)? }),
            1 => Err(unsafe { E::lift(store, options, &src.payload.err)? }),
            _ => bail!("invalid expected discriminant"),
        })
    }

    fn load(memory: &Memory<'_>, bytes: &[u8]) -> Result<Self> {
        debug_assert!((bytes.as_ptr() as usize) % (Self::align() as usize) == 0);
        let align = Self::align();
        let discrim = bytes[0];
        let payload = &bytes[align_to(1, align)..];
        match discrim {
            0 => Ok(Ok(T::load(memory, &payload[..T::size()])?)),
            1 => Ok(Err(E::load(memory, &payload[..E::size()])?)),
            _ => bail!("invalid expected discriminant"),
        }
    }
}

macro_rules! impl_component_ty_for_tuples {
    ($n:tt $($t:ident)*) => {paste::paste!{
        #[allow(non_snake_case)]
        #[doc(hidden)]
        #[derive(Clone, Copy)]
        #[repr(C)]
        pub struct [<TupleLower$n>]<$($t),*> {
            $($t: $t,)*
            _align_tuple_lower0_correctly: [ValRaw; 0],
        }

        #[allow(non_snake_case)]
        unsafe impl<$($t,)*> ComponentType for ($($t,)*)
            where $($t: ComponentType),*
        {
            type Lower = [<TupleLower$n>]<$($t::Lower),*>;

            fn typecheck(
                ty: &InterfaceType,
                types: &ComponentTypes,
            ) -> Result<()> {
                typecheck_tuple(ty, types, &[$($t::typecheck),*])
            }

            #[inline]
            fn size() -> usize {
                let mut _size = 0;
                $(next_field::<$t>(&mut _size);)*
                _size
            }

            #[inline]
            fn align() -> u32 {
                let mut _align = 1;
                $(_align = _align.max($t::align());)*
                _align
            }
        }

        #[allow(non_snake_case)]
        unsafe impl<$($t,)*> Lower for ($($t,)*)
            where $($t: Lower),*
        {
            fn lower<U>(
                &self,
                _store: &mut StoreContextMut<U>,
                _options: &Options,
                _dst: &mut MaybeUninit<Self::Lower>,
            ) -> Result<()> {
                let ($($t,)*) = self;
                $($t.lower(_store, _options, map_maybe_uninit!(_dst.$t))?;)*
                Ok(())
            }

            fn store<U>(&self, _memory: &mut MemoryMut<'_, U>, mut _offset: usize) -> Result<()> {
                debug_assert!(_offset % (Self::align() as usize) == 0);
                let ($($t,)*) = self;
                $($t.store(_memory, next_field::<$t>(&mut _offset))?;)*
                Ok(())
            }
        }

        #[allow(non_snake_case)]
        unsafe impl<$($t,)*> Lift for ($($t,)*)
            where $($t: Lift),*
        {
            fn lift(_store: &StoreOpaque, _options: &Options, _src: &Self::Lower) -> Result<Self> {
                Ok(($($t::lift(_store, _options, &_src.$t)?,)*))
            }

            fn load(_memory: &Memory<'_>, bytes: &[u8]) -> Result<Self> {
                debug_assert!((bytes.as_ptr() as usize) % (Self::align() as usize) == 0);
                let mut _offset = 0;
                $(let $t = $t::load(_memory, &bytes[next_field::<$t>(&mut _offset)..][..$t::size()])?;)*
                Ok(($($t,)*))
            }
        }

        #[allow(non_snake_case)]
        unsafe impl<$($t,)*> ComponentParams for ($($t,)*)
            where $($t: ComponentType),*
        {
            fn typecheck_params(
                params: &[(Option<String>, InterfaceType)],
                _types: &ComponentTypes,
            ) -> Result<()> {
                if params.len() != $n {
                    bail!("expected {} types, found {}", $n, params.len());
                }
                let mut params = params.iter().map(|i| &i.1);
                $($t::typecheck(params.next().unwrap(), _types)?;)*
                debug_assert!(params.next().is_none());
                Ok(())
            }
        }

    }};
}

for_each_function_signature!(impl_component_ty_for_tuples);

fn desc(ty: &InterfaceType) -> &'static str {
    match ty {
        InterfaceType::U8 => "u8",
        InterfaceType::S8 => "s8",
        InterfaceType::U16 => "u16",
        InterfaceType::S16 => "s16",
        InterfaceType::U32 => "u32",
        InterfaceType::S32 => "s32",
        InterfaceType::U64 => "u64",
        InterfaceType::S64 => "s64",
        InterfaceType::Float32 => "f32",
        InterfaceType::Float64 => "f64",
        InterfaceType::Unit => "unit",
        InterfaceType::Bool => "bool",
        InterfaceType::Char => "char",
        InterfaceType::String => "string",
        InterfaceType::List(_) => "list",
        InterfaceType::Tuple(_) => "tuple",
        InterfaceType::Option(_) => "option",
        InterfaceType::Expected(_) => "expected",

        InterfaceType::Record(_) => "record",
        InterfaceType::Variant(_) => "variant",
        InterfaceType::Flags(_) => "flags",
        InterfaceType::Enum(_) => "enum",
        InterfaceType::Union(_) => "union",
    }
}
