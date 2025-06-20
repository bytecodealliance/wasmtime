use crate::component::func::{LiftContext, LowerContext, Options};
use crate::component::matching::InstanceType;
use crate::component::storage::slice_to_storage_mut;
use crate::component::{ComponentNamedList, ComponentType, Instance, Lift, Lower, Val};
use crate::prelude::*;
use crate::runtime::vm::component::{
    ComponentInstance, InstanceFlags, VMComponentContext, VMLowering, VMLoweringCallee,
};
use crate::runtime::vm::{VMFuncRef, VMGlobalDefinition, VMMemoryDefinition, VMOpaqueContext};
use crate::{AsContextMut, CallHook, StoreContextMut, ValRaw};
use alloc::sync::Arc;
use core::any::Any;
use core::mem::{self, MaybeUninit};
use core::ptr::NonNull;
use wasmtime_environ::component::{
    CanonicalAbiInfo, InterfaceType, MAX_FLAT_PARAMS, MAX_FLAT_RESULTS, StringEncoding,
    TypeFuncIndex,
};

pub struct HostFunc {
    entrypoint: VMLoweringCallee,
    typecheck: Box<dyn (Fn(TypeFuncIndex, &InstanceType<'_>) -> Result<()>) + Send + Sync>,
    func: Box<dyn Any + Send + Sync>,
}

impl core::fmt::Debug for HostFunc {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HostFunc").finish_non_exhaustive()
    }
}

impl HostFunc {
    pub(crate) fn from_closure<T, F, P, R>(func: F) -> Arc<HostFunc>
    where
        F: Fn(StoreContextMut<T>, P) -> Result<R> + Send + Sync + 'static,
        P: ComponentNamedList + Lift + 'static,
        R: ComponentNamedList + Lower + 'static,
        T: 'static,
    {
        let entrypoint = Self::entrypoint::<T, F, P, R>;
        Arc::new(HostFunc {
            entrypoint,
            typecheck: Box::new(typecheck::<P, R>),
            func: Box::new(func),
        })
    }

    extern "C" fn entrypoint<T, F, P, R>(
        cx: NonNull<VMOpaqueContext>,
        data: NonNull<u8>,
        ty: u32,
        _caller_instance: u32,
        flags: NonNull<VMGlobalDefinition>,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        async_: u8,
        storage: NonNull<MaybeUninit<ValRaw>>,
        storage_len: usize,
    ) -> bool
    where
        F: Fn(StoreContextMut<T>, P) -> Result<R>,
        P: ComponentNamedList + Lift + 'static,
        R: ComponentNamedList + Lower + 'static,
        T: 'static,
    {
        let data = data.as_ptr() as *const F;
        unsafe {
            call_host_and_handle_result::<T>(cx, |store, instance| {
                call_host(
                    store,
                    instance,
                    TypeFuncIndex::from_u32(ty),
                    InstanceFlags::from_raw(flags),
                    memory,
                    realloc,
                    StringEncoding::from_u8(string_encoding).unwrap(),
                    async_ != 0,
                    NonNull::slice_from_raw_parts(storage, storage_len).as_mut(),
                    |store, args| (*data)(store, args),
                )
            })
        }
    }

    pub(crate) fn new_dynamic<T, F>(func: F) -> Arc<HostFunc>
    where
        F: Fn(StoreContextMut<'_, T>, &[Val], &mut [Val]) -> Result<()> + Send + Sync + 'static,
        T: 'static,
    {
        Arc::new(HostFunc {
            entrypoint: dynamic_entrypoint::<T, F>,
            // This function performs dynamic type checks and subsequently does
            // not need to perform up-front type checks. Instead everything is
            // dynamically managed at runtime.
            typecheck: Box::new(move |_expected_index, _expected_types| Ok(())),
            func: Box::new(func),
        })
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

fn typecheck<P, R>(ty: TypeFuncIndex, types: &InstanceType<'_>) -> Result<()>
where
    P: ComponentNamedList + Lift,
    R: ComponentNamedList + Lower,
{
    let ty = &types.types[ty];
    P::typecheck(&InterfaceType::Tuple(ty.params), types)
        .context("type mismatch with parameters")?;
    R::typecheck(&InterfaceType::Tuple(ty.results), types).context("type mismatch with results")?;
    Ok(())
}

/// The "meat" of calling a host function from wasm.
///
/// This function is delegated to from implementations of
/// `HostFunc::from_closure`. Most of the arguments from the `entrypoint` are
/// forwarded here except for the `data` pointer which is encapsulated in the
/// `closure` argument here.
///
/// This function is parameterized over:
///
/// * `T` - the type of store this function works with (an unsafe assertion)
/// * `Params` - the parameters to the host function, viewed as a tuple
/// * `Return` - the result of the host function
/// * `F` - the `closure` to actually receive the `Params` and return the
///   `Return`
///
/// It's expected that `F` will "un-tuple" the arguments to pass to a host
/// closure.
///
/// This function is in general `unsafe` as the validity of all the parameters
/// must be upheld. Generally that's done by ensuring this is only called from
/// the select few places it's intended to be called from.
unsafe fn call_host<T, Params, Return, F>(
    mut store: StoreContextMut<'_, T>,
    instance: Instance,
    ty: TypeFuncIndex,
    mut flags: InstanceFlags,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: StringEncoding,
    async_: bool,
    storage: &mut [MaybeUninit<ValRaw>],
    closure: F,
) -> Result<()>
where
    Params: Lift,
    Return: Lower,
    F: FnOnce(StoreContextMut<'_, T>, Params) -> Result<Return>,
{
    if async_ {
        todo!()
    }

    let options = Options::new(
        store.0.id(),
        NonNull::new(memory),
        NonNull::new(realloc),
        string_encoding,
    );

    // Perform a dynamic check that this instance can indeed be left. Exiting
    // the component is disallowed, for example, when the `realloc` function
    // calls a canonical import.
    if !flags.may_leave() {
        bail!("cannot leave component instance");
    }

    let types = instance.id().get(store.0).component().types().clone();
    let ty = &types[ty];
    let param_tys = InterfaceType::Tuple(ty.params);
    let result_tys = InterfaceType::Tuple(ty.results);

    let mut storage = Storage::<'_, Params, Return>::new_sync(storage);
    let mut lift = LiftContext::new(store.0, &options, &types, instance);
    lift.enter_call();
    let params = storage.lift_params(&mut lift, param_tys)?;

    let ret = closure(store.as_context_mut(), params)?;

    flags.set_may_leave(false);
    let mut lower = LowerContext::new(store, &options, &types, instance);
    storage.lower_results(&mut lower, result_tys, ret)?;
    flags.set_may_leave(true);
    lower.exit_call()?;

    return Ok(());

    /// Type-level representation of the matrix of possibilities of how
    /// WebAssembly parameters and results are handled in the canonical ABI.
    ///
    /// Wasmtime's ABI here always works with `&mut [MaybeUninit<ValRaw>]` as the
    /// base representation of params/results. Parameters are passed
    /// sequentially and results are returned by overwriting the parameters.
    /// That means both params/results start from index 0.
    ///
    /// The type-level representation here involves working with the typed
    /// `P::Lower` and `R::Lower` values which is a type-level representation of
    /// a lowered value. All lowered values are in essence a sequence of
    /// `ValRaw` values one after the other to fit within this original array
    /// that is the basis of Wasmtime's ABI.
    ///
    /// The various combinations here are cryptic, but only used in this file.
    /// This in theory cuts down on the verbosity below, but an explanation of
    /// the various acronyms here are:
    ///
    /// * Pd - params direct - means that parameters are passed directly in
    ///   their flat representation via `P::Lower`.
    ///
    /// * Pi - params indirect - means that parameters are passed indirectly in
    ///   linear memory and the argument here is `ValRaw` to store the pointer.
    ///
    /// * Rd - results direct - means that results are returned directly in
    ///   their flat representation via `R::Lower`. Note that this is always
    ///   represented as `MaybeUninit<R::Lower>` as well because the return
    ///   values may point to uninitialized memory if there were no parameters
    ///   for example.
    ///
    /// * Ri - results indirect - means that results are returned indirectly in
    ///   linear memory through the pointer specified. Note that this is
    ///   specified as a `ValRaw` to represent the argument that's being given
    ///   to the host from WebAssembly.
    ///
    /// Internally this type makes liberal use of `Union` and `Pair` helpers
    /// below which are simple `#[repr(C)]` wrappers around a pair of types that
    /// are a union or a pair.
    ///
    /// Note that for any combination of `P` and `R` this `enum` is actually
    /// pointless as a single variant will be used. In theory we should be able
    /// to monomorphize based on `P` and `R` to a specific type. This
    /// monomorphization depends on conditionals like `flatten_count() <= N`,
    /// however, and I don't know how to encode that in Rust easily. In lieu of
    /// that we assume LLVM will figure things out and boil away the actual enum
    /// and runtime dispatch.
    enum Storage<'a, P: ComponentType, R: ComponentType> {
        /// Params: direct, Results: direct
        ///
        /// The lowered representation of params/results are overlaid on top of
        /// each other.
        PdRd(&'a mut Union<P::Lower, MaybeUninit<R::Lower>>),

        /// Params: direct, Results: indirect
        ///
        /// The return pointer comes after the params so this is sequentially
        /// laid out with one after the other.
        PdRi(&'a Pair<P::Lower, ValRaw>),

        /// Params: indirect, Results: direct
        ///
        /// Here the return values are overlaid on top of the pointer parameter.
        PiRd(&'a mut Union<ValRaw, MaybeUninit<R::Lower>>),

        /// Params: indirect, Results: indirect
        ///
        /// Here the two parameters are laid out sequentially one after the
        /// other.
        PiRi(&'a Pair<ValRaw, ValRaw>),
    }

    // Helper structure used above in `Storage` to represent two consecutive
    // values.
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct Pair<T, U> {
        a: T,
        b: U,
    }

    // Helper structure used above in `Storage` to represent two values overlaid
    // on each other.
    #[repr(C)]
    union Union<T: Copy, U: Copy> {
        a: T,
        b: U,
    }

    /// Representation of where parameters are lifted from.
    enum Src<'a, T> {
        /// Parameters are directly lifted from `T`, which is under the hood a
        /// sequence of `ValRaw`. This is `P::Lower` for example.
        Direct(&'a T),

        /// Parameters are loaded from linear memory, and this is the wasm
        /// parameter representing the pointer into linear memory to load from.
        Indirect(&'a ValRaw),
    }

    /// Dual of [`Src`], where to store results.
    enum Dst<'a, T> {
        /// Results are stored directly in this pointer.
        ///
        /// Note that this is a mutable pointer but it's specifically
        /// `MaybeUninit` as trampolines do not initialize it. The `T` here will
        /// be `R::Lower` for example.
        Direct(&'a mut MaybeUninit<T>),

        /// Results are stored in linear memory, and this value is the wasm
        /// parameter given which represents the pointer into linear memory.
        ///
        /// Note that this is not mutable as the parameter is not mutated, but
        /// memory will be mutated.
        Indirect(&'a ValRaw),
    }

    impl<P, R> Storage<'_, P, R>
    where
        P: ComponentType + Lift,
        R: ComponentType + Lower,
    {
        /// Classifies a new `Storage` suitable for use with sync functions.
        ///
        /// There's a 2x2 matrix of whether parameters and results are stored on the
        /// stack or on the heap. Each of the 4 branches here have a different
        /// representation of the storage of arguments/returns.
        ///
        /// Also note that while four branches are listed here only one is taken for
        /// any particular `Params` and `Return` combination. This should be
        /// trivially DCE'd by LLVM. Perhaps one day with enough const programming in
        /// Rust we can make monomorphizations of this function codegen only one
        /// branch, but today is not that day.
        ///
        /// # Safety
        ///
        /// Requires that the `storage` provided does indeed match an wasm
        /// function with the signature of `P` and `R` as params/results.
        unsafe fn new_sync(storage: &mut [MaybeUninit<ValRaw>]) -> Storage<'_, P, R> {
            // SAFETY: this `unsafe` is due to the `slice_to_storage_*` helpers
            // used which view the slice provided as a different type. This
            // safety should be upheld by the contract of the `ComponentType`
            // trait and its `Lower` type parameter meaning they're valid to
            // view as a sequence of `ValRaw` types. Additionally the
            // `ComponentType` trait ensures that the matching of the runtime
            // length of `storage` should match the actual size of `P::Lower`
            // and `R::Lower` or such as needed.
            unsafe {
                if P::flatten_count() <= MAX_FLAT_PARAMS {
                    if R::flatten_count() <= MAX_FLAT_RESULTS {
                        Storage::PdRd(slice_to_storage_mut(storage).assume_init_mut())
                    } else {
                        Storage::PdRi(slice_to_storage_mut(storage).assume_init_ref())
                    }
                } else {
                    if R::flatten_count() <= MAX_FLAT_RESULTS {
                        Storage::PiRd(slice_to_storage_mut(storage).assume_init_mut())
                    } else {
                        Storage::PiRi(slice_to_storage_mut(storage).assume_init_ref())
                    }
                }
            }
        }

        fn lift_params(&self, cx: &mut LiftContext<'_>, ty: InterfaceType) -> Result<P> {
            match self.lift_src() {
                Src::Direct(storage) => P::lift(cx, ty, storage),
                Src::Indirect(ptr) => {
                    let ptr = validate_inbounds::<P>(cx.memory(), ptr)?;
                    P::load(cx, ty, &cx.memory()[ptr..][..P::SIZE32])
                }
            }
        }

        fn lift_src(&self) -> Src<'_, P::Lower> {
            match self {
                // SAFETY: these `unsafe` blocks are due to accessing union
                // fields. The safety here relies on the contract of the
                // `ComponentType` trait which should ensure that the types
                // projected onto a list of wasm parameters are indeed correct.
                // That means that the projections here, if the types are
                // correct, all line up to initialized memory that's well-typed
                // to access.
                Storage::PdRd(storage) => unsafe { Src::Direct(&storage.a) },
                Storage::PdRi(storage) => Src::Direct(&storage.a),
                Storage::PiRd(storage) => unsafe { Src::Indirect(&storage.a) },
                Storage::PiRi(storage) => Src::Indirect(&storage.a),
            }
        }

        fn lower_results<T>(
            &mut self,
            cx: &mut LowerContext<'_, T>,
            ty: InterfaceType,
            ret: R,
        ) -> Result<()> {
            match self.lower_dst() {
                Dst::Direct(storage) => ret.lower(cx, ty, storage),
                Dst::Indirect(ptr) => {
                    let ptr = validate_inbounds::<R>(cx.as_slice_mut(), ptr)?;
                    ret.store(cx, ty, ptr)
                }
            }
        }

        fn lower_dst(&mut self) -> Dst<'_, R::Lower> {
            match self {
                // SAFETY: these unsafe blocks are due to accessing fields of a
                // `union` which is not safe in Rust. The returned value is
                // `MaybeUninit<R::Lower>` in all cases, however, which should
                // safely model how `union` memory is possibly uninitialized.
                // Additionally `R::Lower` has the `unsafe` contract that all
                // its bit patterns must be sound, which additionally should
                // help make this safe.
                Storage::PdRd(storage) => unsafe { Dst::Direct(&mut storage.b) },
                Storage::PiRd(storage) => unsafe { Dst::Direct(&mut storage.b) },
                Storage::PdRi(storage) => Dst::Indirect(&storage.b),
                Storage::PiRi(storage) => Dst::Indirect(&storage.b),
            }
        }
    }
}

fn validate_inbounds<T: ComponentType>(memory: &[u8], ptr: &ValRaw) -> Result<usize> {
    // FIXME(#4311): needs memory64 support
    let ptr = usize::try_from(ptr.get_u32())?;
    if ptr % usize::try_from(T::ALIGN32)? != 0 {
        bail!("pointer not aligned");
    }
    let end = match ptr.checked_add(T::SIZE32) {
        Some(n) => n,
        None => bail!("pointer size overflow"),
    };
    if end > memory.len() {
        bail!("pointer out of bounds")
    }
    Ok(ptr)
}

unsafe fn call_host_and_handle_result<T>(
    cx: NonNull<VMOpaqueContext>,
    func: impl FnOnce(StoreContextMut<'_, T>, Instance) -> Result<()>,
) -> bool
where
    T: 'static,
{
    let cx = VMComponentContext::from_opaque(cx);
    ComponentInstance::from_vmctx(cx, |store, instance| {
        let mut store = store.unchecked_context_mut();

        crate::runtime::vm::catch_unwind_and_record_trap(|| {
            store.0.call_hook(CallHook::CallingHost)?;
            let res = func(store.as_context_mut(), instance);
            store.0.call_hook(CallHook::ReturningFromHost)?;
            res
        })
    })
}

unsafe fn call_host_dynamic<T, F>(
    mut store: StoreContextMut<'_, T>,
    instance: Instance,
    ty: TypeFuncIndex,
    mut flags: InstanceFlags,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: StringEncoding,
    async_: bool,
    storage: &mut [MaybeUninit<ValRaw>],
    closure: F,
) -> Result<()>
where
    F: FnOnce(StoreContextMut<'_, T>, &[Val], &mut [Val]) -> Result<()>,
    T: 'static,
{
    if async_ {
        todo!()
    }

    let options = Options::new(
        store.0.id(),
        NonNull::new(memory),
        NonNull::new(realloc),
        string_encoding,
    );

    // Perform a dynamic check that this instance can indeed be left. Exiting
    // the component is disallowed, for example, when the `realloc` function
    // calls a canonical import.
    if !flags.may_leave() {
        bail!("cannot leave component instance");
    }

    let args;
    let ret_index;

    let types = instance.id().get(store.0).component().types().clone();
    let func_ty = &types[ty];
    let param_tys = &types[func_ty.params];
    let result_tys = &types[func_ty.results];
    let mut cx = LiftContext::new(store.0, &options, &types, instance);
    cx.enter_call();
    if let Some(param_count) = param_tys.abi.flat_count(MAX_FLAT_PARAMS) {
        // NB: can use `MaybeUninit::slice_assume_init_ref` when that's stable
        let mut iter =
            mem::transmute::<&[MaybeUninit<ValRaw>], &[ValRaw]>(&storage[..param_count]).iter();
        args = param_tys
            .types
            .iter()
            .map(|ty| Val::lift(&mut cx, *ty, &mut iter))
            .collect::<Result<Box<[_]>>>()?;
        ret_index = param_count;
        assert!(iter.next().is_none());
    } else {
        let mut offset =
            validate_inbounds_dynamic(&param_tys.abi, cx.memory(), storage[0].assume_init_ref())?;
        args = param_tys
            .types
            .iter()
            .map(|ty| {
                let abi = types.canonical_abi(ty);
                let size = usize::try_from(abi.size32).unwrap();
                let memory = &cx.memory()[abi.next_field32_size(&mut offset)..][..size];
                Val::load(&mut cx, *ty, memory)
            })
            .collect::<Result<Box<[_]>>>()?;
        ret_index = 1;
    };

    let mut result_vals = Vec::with_capacity(result_tys.types.len());
    for _ in result_tys.types.iter() {
        result_vals.push(Val::Bool(false));
    }
    closure(store.as_context_mut(), &args, &mut result_vals)?;
    flags.set_may_leave(false);

    let mut cx = LowerContext::new(store, &options, &types, instance);
    if let Some(cnt) = result_tys.abi.flat_count(MAX_FLAT_RESULTS) {
        let mut dst = storage[..cnt].iter_mut();
        for (val, ty) in result_vals.iter().zip(result_tys.types.iter()) {
            val.lower(&mut cx, *ty, &mut dst)?;
        }
        assert!(dst.next().is_none());
    } else {
        let ret_ptr = storage[ret_index].assume_init_ref();
        let mut ptr = validate_inbounds_dynamic(&result_tys.abi, cx.as_slice_mut(), ret_ptr)?;
        for (val, ty) in result_vals.iter().zip(result_tys.types.iter()) {
            let offset = types.canonical_abi(ty).next_field32_size(&mut ptr);
            val.store(&mut cx, *ty, offset)?;
        }
    }

    flags.set_may_leave(true);

    cx.exit_call()?;

    return Ok(());
}

fn validate_inbounds_dynamic(abi: &CanonicalAbiInfo, memory: &[u8], ptr: &ValRaw) -> Result<usize> {
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

extern "C" fn dynamic_entrypoint<T, F>(
    cx: NonNull<VMOpaqueContext>,
    data: NonNull<u8>,
    ty: u32,
    _caller_instance: u32,
    flags: NonNull<VMGlobalDefinition>,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    async_: u8,
    storage: NonNull<MaybeUninit<ValRaw>>,
    storage_len: usize,
) -> bool
where
    F: Fn(StoreContextMut<'_, T>, &[Val], &mut [Val]) -> Result<()> + Send + Sync + 'static,
    T: 'static,
{
    let data = data.as_ptr() as *const F;
    unsafe {
        call_host_and_handle_result(cx, |store, instance| {
            call_host_dynamic::<T, _>(
                store,
                instance,
                TypeFuncIndex::from_u32(ty),
                InstanceFlags::from_raw(flags),
                memory,
                realloc,
                StringEncoding::from_u8(string_encoding).unwrap(),
                async_ != 0,
                NonNull::slice_from_raw_parts(storage, storage_len).as_mut(),
                |store, params, results| (*data)(store, params, results),
            )
        })
    }
}
