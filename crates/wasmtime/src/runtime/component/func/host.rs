//! Implementation of calling Rust-defined functions from components.

use crate::component::concurrent;
#[cfg(feature = "component-model-async")]
use crate::component::concurrent::{Accessor, Status};
use crate::component::func::{LiftContext, LowerContext};
use crate::component::matching::InstanceType;
use crate::component::storage::{slice_to_storage, slice_to_storage_mut};
use crate::component::types::ComponentFunc;
use crate::component::{ComponentNamedList, Instance, Lift, Lower, Val};
use crate::prelude::*;
use crate::runtime::vm::component::{
    ComponentInstance, VMComponentContext, VMLowering, VMLoweringCallee,
};
use crate::runtime::vm::{VMOpaqueContext, VMStore};
use crate::{AsContextMut, CallHook, StoreContextMut, ValRaw};
use alloc::sync::Arc;
use core::any::Any;
use core::mem::{self, MaybeUninit};
#[cfg(feature = "component-model-async")]
use core::pin::Pin;
use core::ptr::NonNull;
use wasmtime_environ::component::{
    CanonicalAbiInfo, InterfaceType, MAX_FLAT_PARAMS, MAX_FLAT_RESULTS, OptionsIndex, TypeFuncIndex,
};

/// A host function suitable for passing into a component.
///
/// This structure represents a monomorphic host function that can only be used
/// in the specific context of a particular store. This is generally not too
/// too safe to use and is only meant for internal use.
pub struct HostFunc {
    /// The raw function pointer which Cranelift will invoke.
    entrypoint: VMLoweringCallee,

    /// The implementation of type-checking to ensure that this function
    /// ascribes to the provided function type.
    ///
    /// This is used, for example, when a component imports a host function and
    /// this will determine if the host function can be imported with the given
    /// type.
    typecheck: fn(TypeFuncIndex, &InstanceType<'_>) -> Result<()>,

    /// The actual host function.
    ///
    /// This is frequently an empty allocation in the sense that the underlying
    /// type is a zero-sized-type. Host functions are allowed, though, to close
    /// over the environment as well.
    func: Box<dyn Any + Send + Sync>,
}

impl core::fmt::Debug for HostFunc {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HostFunc").finish_non_exhaustive()
    }
}

enum HostResult<T> {
    Done(Result<T>),
    #[cfg(feature = "component-model-async")]
    Future(Pin<Box<dyn Future<Output = Result<T>> + Send>>),
}

impl HostFunc {
    fn new<T, F, P, R>(func: F) -> Arc<HostFunc>
    where
        T: 'static,
        R: Send + Sync + 'static,
        F: HostFn<T, P, R> + Send + Sync + 'static,
    {
        Arc::new(HostFunc {
            entrypoint: F::cabi_entrypoint,
            typecheck: F::typecheck,
            func: Box::new(func),
        })
    }

    /// Creates a new, statically typed, synchronous, host function from the
    /// `func` provided.
    pub(crate) fn from_closure<T, F, P, R>(func: F) -> Arc<HostFunc>
    where
        T: 'static,
        F: Fn(StoreContextMut<T>, P) -> Result<R> + Send + Sync + 'static,
        P: ComponentNamedList + Lift + 'static,
        R: ComponentNamedList + Lower + 'static,
    {
        Self::new(StaticHostFn::<_, false>::new(move |store, params| {
            HostResult::Done(func(store, params))
        }))
    }

    /// Creates a new, statically typed, asynchronous, host function from the
    /// `func` provided.
    #[cfg(feature = "component-model-async")]
    pub(crate) fn from_concurrent<T, F, P, R>(func: F) -> Arc<HostFunc>
    where
        T: 'static,
        F: Fn(&Accessor<T>, P) -> Pin<Box<dyn Future<Output = Result<R>> + Send + '_>>
            + Send
            + Sync
            + 'static,
        P: ComponentNamedList + Lift + 'static,
        R: ComponentNamedList + Lower + 'static,
    {
        let func = Arc::new(func);
        Self::new(StaticHostFn::<_, true>::new(move |store, params| {
            let func = func.clone();
            HostResult::Future(Box::pin(
                store.wrap_call(move |accessor| func(accessor, params)),
            ))
        }))
    }

    /// Creates a new, dynamically typed, synchronous, host function from the
    /// `func` provided.
    pub(crate) fn new_dynamic<T: 'static, F>(func: F) -> Arc<HostFunc>
    where
        F: Fn(StoreContextMut<'_, T>, ComponentFunc, &[Val], &mut [Val]) -> Result<()>
            + Send
            + Sync
            + 'static,
    {
        Self::new(DynamicHostFn::<_, false>::new(
            move |store, ty, mut params_and_results, result_start| {
                let (params, results) = params_and_results.split_at_mut(result_start);
                let result = func(store, ty, params, results).map(move |()| params_and_results);
                HostResult::Done(result)
            },
        ))
    }

    /// Creates a new, dynamically typed, asynchronous, host function from the
    /// `func` provided.
    #[cfg(feature = "component-model-async")]
    pub(crate) fn new_dynamic_concurrent<T, F>(func: F) -> Arc<HostFunc>
    where
        T: 'static,
        F: for<'a> Fn(
                &'a Accessor<T>,
                ComponentFunc,
                &'a [Val],
                &'a mut [Val],
            ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
            + Send
            + Sync
            + 'static,
    {
        let func = Arc::new(func);
        Self::new(DynamicHostFn::<_, true>::new(
            move |store, ty, mut params_and_results, result_start| {
                let func = func.clone();
                HostResult::Future(Box::pin(store.wrap_call(move |accessor| {
                    Box::pin(async move {
                        let (params, results) = params_and_results.split_at_mut(result_start);
                        func(accessor, ty, params, results).await?;
                        Ok(params_and_results)
                    })
                })))
            },
        ))
    }

    pub fn typecheck(&self, ty: TypeFuncIndex, types: &InstanceType<'_>) -> Result<()> {
        (self.typecheck)(ty, types)
    }

    pub fn lowering(&self) -> VMLowering {
        let data = NonNull::from(&*self.func).cast();
        VMLowering {
            callee: NonNull::new(self.entrypoint as *mut _).unwrap().into(),
            data: data.into(),
        }
    }
}

/// Argument to [`HostFn::lift_params`]
enum Source<'a> {
    /// The parameters come from flat wasm arguments which are provided here.
    Flat(&'a [ValRaw]),
    /// The parameters come from linear memory at the provided offset, which is
    /// already validated to be in-bounds.
    Memory(usize),
}

/// Argument to [`HostFn::lower_result`]
enum Destination<'a> {
    /// The result is stored in flat parameters whose storage is provided here.
    Flat(&'a mut [MaybeUninit<ValRaw>]),
    /// The result is stored in linear memory at the provided offset, which is
    /// already validated to be in-bounds.
    Memory(usize),
}

/// Consolidation of functionality of invoking a host function.
///
/// This trait primarily serves as a deduplication of the "static" and
/// "dynamic" host function paths where all default functions here are shared
/// (source-wise at least) across the two styles of host functions.
trait HostFn<T, P, R>
where
    T: 'static,
    R: Send + Sync + 'static,
{
    /// Whether or not this is `async` function from the perspective of the
    /// component model.
    const ASYNC: bool;

    /// Performs a type-check to ensure that this host function can be imported
    /// with the provided signature that a component is using.
    fn typecheck(ty: TypeFuncIndex, types: &InstanceType<'_>) -> Result<()>;

    /// Execute this host function.
    fn run(&self, store: StoreContextMut<'_, T>, params: P) -> HostResult<R>;

    /// Performs the lifting operation to convert arguments from the canonical
    /// ABI in wasm memory/arguments into their Rust representation.
    fn lift_params(cx: &mut LiftContext<'_>, ty: TypeFuncIndex, source: Source<'_>) -> Result<P>;

    /// Performs the lowering operation to convert the result from its Rust
    /// representation to the canonical ABI representation.
    fn lower_result(
        cx: &mut LowerContext<'_, T>,
        ty: TypeFuncIndex,
        result: R,
        dst: Destination<'_>,
    ) -> Result<()>;

    /// Raw entrypoint invoked by Cranelift.
    ///
    /// # Safety
    ///
    /// This function is only safe when called from a trusted source which
    /// upholds at least these invariants:
    ///
    /// * `cx` is a valid pointer which comes from calling wasm.
    /// * `data` is a valid pointer to `Self`
    /// * `ty` and `options` are valid within the context of `cx`
    /// * `storage` and `storage_len` are valid pointers and correspond to
    ///   correctly initialized wasm arguments/results according to the
    ///   canonical ABI specified by `ty` and `options`.
    ///
    /// The code elsewhere in this trait is all downstream of this `unsafe`,
    /// and upholding this `unsafe` invariant requires Cranelift, function
    /// translation, the canonical ABI, and Wasmtime to all stay in sync.
    /// Basically we can't statically rule out this `unsafe`, we just gotta
    /// not have bugs.
    unsafe extern "C" fn cabi_entrypoint(
        cx: NonNull<VMOpaqueContext>,
        data: NonNull<u8>,
        ty: u32,
        options: u32,
        storage: NonNull<MaybeUninit<ValRaw>>,
        storage_len: usize,
    ) -> bool
    where
        Self: Sized,
    {
        let cx = unsafe { VMComponentContext::from_opaque(cx) };
        unsafe {
            ComponentInstance::enter_host_from_wasm(cx, |store, instance| {
                let mut store = store.unchecked_context_mut();
                let ty = TypeFuncIndex::from_u32(ty);
                let options = OptionsIndex::from_u32(options);
                let storage = NonNull::slice_from_raw_parts(storage, storage_len).as_mut();
                let data = data.cast::<Self>().as_ref();

                store.0.call_hook(CallHook::CallingHost)?;
                let res = data.entrypoint(store.as_context_mut(), instance, ty, options, storage);
                store.0.call_hook(CallHook::ReturningFromHost)?;

                res
            })
        }
    }

    /// "Rust" entrypoint after panic-handling infrastructure is set up and raw
    /// arguments are translated to Rust types.
    fn entrypoint(
        &self,
        store: StoreContextMut<'_, T>,
        instance: Instance,
        ty: TypeFuncIndex,
        options: OptionsIndex,
        storage: &mut [MaybeUninit<ValRaw>],
    ) -> Result<()> {
        let vminstance = instance.id().get(store.0);
        let opts = &vminstance.component().env_component().options[options];
        let caller_instance = opts.instance;
        let flags = vminstance.instance_flags(caller_instance);

        // Perform a dynamic check that this instance can indeed be left.
        // Exiting the component is disallowed, for example, when the `realloc`
        // function calls a canonical import.
        if unsafe { !flags.may_leave() } {
            return Err(anyhow!(crate::Trap::CannotLeaveComponent));
        }

        if opts.async_ {
            #[cfg(feature = "component-model-async")]
            return self.call_async_lower(store, instance, ty, options, storage);
            #[cfg(not(feature = "component-model-async"))]
            unreachable!(
                "async-lowered imports should have failed validation \
                 when `component-model-async` feature disabled"
            );
        } else {
            self.call_sync_lower(store, instance, ty, options, storage)
        }
    }

    /// Implementation of the "sync" ABI.
    ///
    /// This is the implementation of invoking a host function through the
    /// synchronous ABI of the component model, or when a function doesn't have
    /// the `async` option when lowered. Note that the host function itself
    /// can still be async, in which case this will block here waiting for it
    /// to finish.
    fn call_sync_lower(
        &self,
        mut store: StoreContextMut<'_, T>,
        instance: Instance,
        ty: TypeFuncIndex,
        options: OptionsIndex,
        storage: &mut [MaybeUninit<ValRaw>],
    ) -> Result<()> {
        if Self::ASYNC {
            // The caller has synchronously lowered an async function, meaning
            // the caller can only call it from an async task (i.e. a task
            // created via a call to an async export).  Otherwise, we'll trap.
            concurrent::check_blocking(store.0)?;
        }

        let mut lift = LiftContext::new(store.0.store_opaque_mut(), options, instance);
        let (params, rest) = self.load_params(&mut lift, ty, MAX_FLAT_PARAMS, storage)?;
        #[cfg(feature = "component-model-async")]
        let caller_instance = lift.options().instance;

        let ret = match self.run(store.as_context_mut(), params) {
            HostResult::Done(result) => result?,
            #[cfg(feature = "component-model-async")]
            HostResult::Future(future) => {
                concurrent::poll_and_block(store.0, future, caller_instance)?
            }
        };

        let mut lower = LowerContext::new(store, options, instance);
        let fty = &lower.types[ty];
        let result_tys = &lower.types[fty.results];
        let dst = if let Some(cnt) = result_tys.abi.flat_count(MAX_FLAT_RESULTS) {
            Destination::Flat(&mut storage[..cnt])
        } else {
            // SAFETY: due to the contract of `entrypoint` we know that the
            // return pointer, located after the parameters, is initialized
            // by wasm and safe to read.
            let ptr = unsafe { rest[0].assume_init_ref() };
            Destination::Memory(validate_inbounds_dynamic(
                &result_tys.abi,
                lower.as_slice_mut(),
                ptr,
            )?)
        };
        Self::lower_result_and_exit_call(&mut lower, ty, ret, dst)
    }

    /// Implementation of the "async" ABI of the component model.
    ///
    /// This is invoked when a component has the `async` options specified on
    /// its `canon lower` for a host function. Note that the host function may
    /// be either sync or async, and that's handled here too.
    #[cfg(feature = "component-model-async")]
    fn call_async_lower(
        &self,
        store: StoreContextMut<'_, T>,
        instance: Instance,
        ty: TypeFuncIndex,
        options: OptionsIndex,
        storage: &mut [MaybeUninit<ValRaw>],
    ) -> Result<()> {
        use wasmtime_environ::component::MAX_FLAT_ASYNC_PARAMS;

        let (component, store) = instance.component_and_store_mut(store.0);
        let mut store = StoreContextMut(store);
        let types = component.types();
        let fty = &types[ty];

        // Lift the parameters, either from flat storage or from linear
        // memory.
        let mut lift = LiftContext::new(store.0.store_opaque_mut(), options, instance);
        let caller_instance = lift.options().instance;
        let (params, rest) = self.load_params(&mut lift, ty, MAX_FLAT_ASYNC_PARAMS, storage)?;

        // Load/validate the return pointer, if present.
        let retptr = if !lift.types[fty.results].types.is_empty() {
            let mut lower = LowerContext::new(store.as_context_mut(), options, instance);
            // SAFETY: see `load_params` below about how the return pointer
            // should be safe to use.
            let ptr = unsafe { rest[0].assume_init_ref() };
            let result_tys = &lower.types[fty.results];
            validate_inbounds_dynamic(&result_tys.abi, lower.as_slice_mut(), ptr)?
        } else {
            // If there's no return pointer then `R` should have an
            // empty flat representation. In this situation pretend the return
            // pointer was 0 so we have something to shepherd along into the
            // closure below.
            0
        };

        let host_result = self.run(store.as_context_mut(), params);

        let task = match host_result {
            HostResult::Done(result) => {
                Self::lower_result_and_exit_call(
                    &mut LowerContext::new(store, options, instance),
                    ty,
                    result?,
                    Destination::Memory(retptr),
                )?;
                None
            }
            #[cfg(feature = "component-model-async")]
            HostResult::Future(future) => {
                instance.first_poll(store, future, caller_instance, move |store, ret| {
                    Self::lower_result_and_exit_call(
                        &mut LowerContext::new(store, options, instance),
                        ty,
                        ret,
                        Destination::Memory(retptr),
                    )
                })?
            }
        };

        storage[0].write(ValRaw::u32(if let Some(task) = task {
            Status::Started.pack(Some(task))
        } else {
            Status::Returned.pack(None)
        }));

        Ok(())
    }

    /// Loads parameters the wasm arguments `storage`.
    ///
    /// This will internally decide the ABI source of the parameters and use
    /// `storage` appropriately.
    fn load_params<'a>(
        &self,
        lift: &mut LiftContext<'_>,
        ty: TypeFuncIndex,
        max_flat_params: usize,
        storage: &'a [MaybeUninit<ValRaw>],
    ) -> Result<(P, &'a [MaybeUninit<ValRaw>])> {
        let fty = &lift.types[ty];
        let param_tys = &lift.types[fty.params];
        let param_flat_count = param_tys.abi.flat_count(max_flat_params);
        lift.enter_call();
        let src = match param_flat_count {
            Some(cnt) => {
                let params = &storage[..cnt];
                // SAFETY: due to the contract of `entrypoint` we are
                // guaranteed that all flat parameters are initialized by
                // compiled wasm.
                Source::Flat(unsafe { mem::transmute::<&[MaybeUninit<ValRaw>], &[ValRaw]>(params) })
            }
            None => {
                // SAFETY: due to the contract of `entrypoint` we are
                // guaranteed that the return pointer is initialized by
                // compiled wasm.
                let ptr = unsafe { storage[0].assume_init_ref() };
                Source::Memory(validate_inbounds_dynamic(
                    &param_tys.abi,
                    lift.memory(),
                    ptr,
                )?)
            }
        };
        let params = Self::lift_params(lift, ty, src)?;
        Ok((params, &storage[param_flat_count.unwrap_or(1)..]))
    }

    /// Stores the result `ret` into `dst` which is calculated per the ABI.
    fn lower_result_and_exit_call(
        lower: &mut LowerContext<'_, T>,
        ty: TypeFuncIndex,
        ret: R,
        dst: Destination<'_>,
    ) -> Result<()> {
        let caller_instance = lower.options().instance;
        let mut flags = lower.instance_mut().instance_flags(caller_instance);
        unsafe {
            flags.set_may_leave(false);
        }
        Self::lower_result(lower, ty, ret, dst)?;
        unsafe {
            flags.set_may_leave(true);
        }
        lower.exit_call()?;
        Ok(())
    }
}

/// Implementation of a "static" host function where the parameters and results
/// of a function are known at compile time.
#[repr(transparent)]
struct StaticHostFn<F, const ASYNC: bool>(F);

impl<F, const ASYNC: bool> StaticHostFn<F, ASYNC> {
    fn new<T, P, R>(func: F) -> Self
    where
        T: 'static,
        P: ComponentNamedList + Lift + 'static,
        R: ComponentNamedList + Lower + 'static,
        F: Fn(StoreContextMut<'_, T>, P) -> HostResult<R>,
    {
        Self(func)
    }
}

impl<T, F, P, R, const ASYNC: bool> HostFn<T, P, R> for StaticHostFn<F, ASYNC>
where
    T: 'static,
    F: Fn(StoreContextMut<'_, T>, P) -> HostResult<R>,
    P: ComponentNamedList + Lift + 'static,
    R: ComponentNamedList + Lower + 'static,
{
    const ASYNC: bool = ASYNC;

    fn typecheck(ty: TypeFuncIndex, types: &InstanceType<'_>) -> Result<()> {
        let ty = &types.types[ty];
        if ASYNC != ty.async_ {
            bail!("type mismatch with async");
        }
        P::typecheck(&InterfaceType::Tuple(ty.params), types)
            .context("type mismatch with parameters")?;
        R::typecheck(&InterfaceType::Tuple(ty.results), types)
            .context("type mismatch with results")?;
        Ok(())
    }

    fn run(&self, store: StoreContextMut<'_, T>, params: P) -> HostResult<R> {
        (self.0)(store, params)
    }

    fn lift_params(cx: &mut LiftContext<'_>, ty: TypeFuncIndex, src: Source<'_>) -> Result<P> {
        let ty = InterfaceType::Tuple(cx.types[ty].params);
        match src {
            Source::Flat(storage) => {
                // SAFETY: the contract of `ComponentType` for `P` means that
                // it's safe to interpret the parameters `storage` as
                // `P::Lower`. The contract of `entrypoint` is that everything
                // is initialized correctly internally.
                let storage: &P::Lower = unsafe { slice_to_storage(storage) };
                P::linear_lift_from_flat(cx, ty, storage)
            }
            Source::Memory(offset) => {
                P::linear_lift_from_memory(cx, ty, &cx.memory()[offset..][..P::SIZE32])
            }
        }
    }

    fn lower_result(
        cx: &mut LowerContext<'_, T>,
        ty: TypeFuncIndex,
        ret: R,
        dst: Destination<'_>,
    ) -> Result<()> {
        let fty = &cx.types[ty];
        let ty = InterfaceType::Tuple(fty.results);
        match dst {
            Destination::Flat(storage) => {
                // SAFETY: the contract of `ComponentType` for `R` means that
                // it's safe to reinterpret `ValRaw` storage to initialize as
                // `R::Lower`.
                let storage: &mut MaybeUninit<R::Lower> = unsafe { slice_to_storage_mut(storage) };
                ret.linear_lower_to_flat(cx, ty, storage)
            }
            Destination::Memory(ptr) => ret.linear_lower_to_memory(cx, ty, ptr),
        }
    }
}

/// Implementation of a "dynamic" host function where the number of parameters,
/// types of parameters, and result type/presence, are all not known at compile
/// time.
///
/// This is intended for more-dynamic use cases than `StaticHostFn` above such
/// as demos, gluing things together quickly, and `wast` testing.
struct DynamicHostFn<F, const ASYNC: bool>(F);

impl<F, const ASYNC: bool> DynamicHostFn<F, ASYNC> {
    fn new<T>(func: F) -> Self
    where
        T: 'static,
        F: Fn(StoreContextMut<'_, T>, ComponentFunc, Vec<Val>, usize) -> HostResult<Vec<Val>>,
    {
        Self(func)
    }
}

impl<T, F, const ASYNC: bool> HostFn<T, (ComponentFunc, Vec<Val>), Vec<Val>>
    for DynamicHostFn<F, ASYNC>
where
    T: 'static,
    F: Fn(StoreContextMut<'_, T>, ComponentFunc, Vec<Val>, usize) -> HostResult<Vec<Val>>,
{
    const ASYNC: bool = ASYNC;

    /// This function performs dynamic type checks on its parameters and
    /// results and subsequently does not need to perform up-front type
    /// checks. However, we _do_ verify async-ness here.
    fn typecheck(ty: TypeFuncIndex, types: &InstanceType<'_>) -> Result<()> {
        let ty = &types.types[ty];
        if ASYNC != ty.async_ {
            bail!("type mismatch with async");
        }

        Ok(())
    }

    fn run(
        &self,
        store: StoreContextMut<'_, T>,
        (ty, mut params): (ComponentFunc, Vec<Val>),
    ) -> HostResult<Vec<Val>> {
        let offset = params.len();
        for _ in 0..ty.results().len() {
            params.push(Val::Bool(false));
        }
        (self.0)(store, ty, params, offset)
    }

    fn lift_params(
        cx: &mut LiftContext<'_>,
        ty: TypeFuncIndex,
        src: Source<'_>,
    ) -> Result<(ComponentFunc, Vec<Val>)> {
        let param_tys = &cx.types[cx.types[ty].params];
        let mut params = Vec::new();
        match src {
            Source::Flat(storage) => {
                let mut iter = storage.iter();
                for ty in param_tys.types.iter() {
                    params.push(Val::lift(cx, *ty, &mut iter)?);
                }
                assert!(iter.next().is_none());
            }
            Source::Memory(mut offset) => {
                for ty in param_tys.types.iter() {
                    let abi = cx.types.canonical_abi(ty);
                    let size = usize::try_from(abi.size32).unwrap();
                    let memory = &cx.memory()[abi.next_field32_size(&mut offset)..][..size];
                    params.push(Val::load(cx, *ty, memory)?);
                }
            }
        }

        Ok((ComponentFunc::from(ty, &cx.instance_type()), params))
    }

    fn lower_result(
        cx: &mut LowerContext<'_, T>,
        ty: TypeFuncIndex,
        result_vals: Vec<Val>,
        dst: Destination<'_>,
    ) -> Result<()> {
        let fty = &cx.types[ty];
        let param_tys = &cx.types[fty.params];
        let result_tys = &cx.types[fty.results];
        let result_vals = &result_vals[param_tys.types.len()..];
        match dst {
            Destination::Flat(storage) => {
                let mut dst = storage.iter_mut();
                for (val, ty) in result_vals.iter().zip(result_tys.types.iter()) {
                    val.lower(cx, *ty, &mut dst)?;
                }
                assert!(dst.next().is_none());
            }
            Destination::Memory(mut ptr) => {
                for (val, ty) in result_vals.iter().zip(result_tys.types.iter()) {
                    let offset = cx.types.canonical_abi(ty).next_field32_size(&mut ptr);
                    val.store(cx, *ty, offset)?;
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_inbounds_dynamic(
    abi: &CanonicalAbiInfo,
    memory: &[u8],
    ptr: &ValRaw,
) -> Result<usize> {
    // FIXME(#4311): needs memory64 support
    let ptr = usize::try_from(ptr.get_u32())?;
    if ptr % usize::try_from(abi.align32)? != 0 {
        bail!("pointer not aligned");
    }
    let end = match ptr.checked_add(usize::try_from(abi.size32).unwrap()) {
        Some(n) => n,
        None => bail!("pointer size overflow"),
    };
    if end > memory.len() {
        bail!("pointer out of bounds")
    }
    Ok(ptr)
}
