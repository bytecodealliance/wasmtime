use crate::component::func::{Memory, MemoryMut, Options};
use crate::component::storage::slice_to_storage_mut;
use crate::component::{ComponentNamedList, ComponentType, Lift, Lower, Type, Val};
use crate::{AsContextMut, StoreContextMut, ValRaw};
use anyhow::{anyhow, bail, Context, Result};
use std::any::Any;
use std::mem::{self, MaybeUninit};
use std::panic::{self, AssertUnwindSafe};
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::component::{
    CanonicalAbiInfo, ComponentTypes, StringEncoding, TypeFuncIndex, MAX_FLAT_PARAMS,
    MAX_FLAT_RESULTS,
};
use wasmtime_runtime::component::{
    InstanceFlags, VMComponentContext, VMLowering, VMLoweringCallee,
};
use wasmtime_runtime::{VMCallerCheckedAnyfunc, VMMemoryDefinition, VMOpaqueContext};

pub struct HostFunc {
    entrypoint: VMLoweringCallee,
    typecheck: Box<dyn (Fn(TypeFuncIndex, &Arc<ComponentTypes>) -> Result<()>) + Send + Sync>,
    func: Box<dyn Any + Send + Sync>,
}

impl HostFunc {
    pub(crate) fn from_closure<T, F, P, R>(func: F) -> Arc<HostFunc>
    where
        F: Fn(StoreContextMut<T>, P) -> Result<R> + Send + Sync + 'static,
        P: ComponentNamedList + Lift + 'static,
        R: ComponentNamedList + Lower + 'static,
    {
        let entrypoint = Self::entrypoint::<T, F, P, R>;
        Arc::new(HostFunc {
            entrypoint,
            typecheck: Box::new(typecheck::<P, R>),
            func: Box::new(func),
        })
    }

    extern "C" fn entrypoint<T, F, P, R>(
        cx: *mut VMOpaqueContext,
        data: *mut u8,
        flags: InstanceFlags,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMCallerCheckedAnyfunc,
        string_encoding: StringEncoding,
        storage: *mut ValRaw,
        storage_len: usize,
    ) where
        F: Fn(StoreContextMut<T>, P) -> Result<R>,
        P: ComponentNamedList + Lift + 'static,
        R: ComponentNamedList + Lower + 'static,
    {
        let data = data as *const F;
        unsafe {
            handle_result(|| {
                call_host::<_, _, _, _>(
                    cx,
                    flags,
                    memory,
                    realloc,
                    string_encoding,
                    std::slice::from_raw_parts_mut(storage, storage_len),
                    |store, args| (*data)(store, args),
                )
            })
        }
    }

    pub(crate) fn new_dynamic<T, F>(
        func: F,
        index: TypeFuncIndex,
        types: &Arc<ComponentTypes>,
    ) -> Arc<HostFunc>
    where
        F: Fn(StoreContextMut<'_, T>, &[Val], &mut [Val]) -> Result<()> + Send + Sync + 'static,
    {
        let ty = &types[index];

        Arc::new(HostFunc {
            entrypoint: dynamic_entrypoint::<T, F>,
            typecheck: Box::new({
                let types = types.clone();

                move |expected_index, expected_types| {
                    if index == expected_index && Arc::ptr_eq(&types, expected_types) {
                        Ok(())
                    } else {
                        Err(anyhow!("function type mismatch"))
                    }
                }
            }),
            func: Box::new(DynamicContext {
                func,
                types: Types {
                    params: ty.params.iter().map(|ty| Type::from(ty, types)).collect(),
                    results: ty.results.iter().map(|ty| Type::from(ty, types)).collect(),
                },
            }),
        })
    }

    pub fn typecheck(&self, ty: TypeFuncIndex, types: &Arc<ComponentTypes>) -> Result<()> {
        (self.typecheck)(ty, types)
    }

    pub fn lowering(&self) -> VMLowering {
        let data = &*self.func as *const (dyn Any + Send + Sync) as *mut u8;
        VMLowering {
            callee: self.entrypoint,
            data,
        }
    }
}

fn typecheck<P, R>(ty: TypeFuncIndex, types: &Arc<ComponentTypes>) -> Result<()>
where
    P: ComponentNamedList + Lift,
    R: ComponentNamedList + Lower,
{
    let ty = &types[ty];
    P::typecheck_list(&ty.params, types).context("type mismatch with parameters")?;
    R::typecheck_list(&ty.results, types).context("type mismatch with results")?;
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
    cx: *mut VMOpaqueContext,
    mut flags: InstanceFlags,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMCallerCheckedAnyfunc,
    string_encoding: StringEncoding,
    storage: &mut [ValRaw],
    closure: F,
) -> Result<()>
where
    Params: Lift,
    Return: Lower,
    F: FnOnce(StoreContextMut<'_, T>, Params) -> Result<Return>,
{
    /// Representation of arguments to this function when a return pointer is in
    /// use, namely the argument list is followed by a single value which is the
    /// return pointer.
    #[repr(C)]
    struct ReturnPointer<T> {
        args: T,
        retptr: ValRaw,
    }

    /// Representation of arguments to this function when the return value is
    /// returned directly, namely the arguments and return value all start from
    /// the beginning (aka this is a `union`, not a `struct`).
    #[repr(C)]
    union ReturnStack<T: Copy, U: Copy> {
        args: T,
        ret: U,
    }

    let cx = VMComponentContext::from_opaque(cx);
    let instance = (*cx).instance();
    let mut cx = StoreContextMut::from_raw((*instance).store());

    let options = Options::new(
        cx.0.id(),
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

    // There's a 2x2 matrix of whether parameters and results are stored on the
    // stack or on the heap. Each of the 4 branches here have a different
    // representation of the storage of arguments/returns which is represented
    // by the type parameter that we pass to `slice_to_storage_mut`.
    //
    // Also note that while four branches are listed here only one is taken for
    // any particular `Params` and `Return` combination. This should be
    // trivially DCE'd by LLVM. Perhaps one day with enough const programming in
    // Rust we can make monomorphizations of this function codegen only one
    // branch, but today is not that day.
    if Params::flatten_count() <= MAX_FLAT_PARAMS {
        if Return::flatten_count() <= MAX_FLAT_RESULTS {
            let storage =
                slice_to_storage_mut::<ReturnStack<Params::Lower, Return::Lower>>(storage);
            let params = Params::lift(cx.0, &options, &storage.assume_init_ref().args)?;
            let ret = closure(cx.as_context_mut(), params)?;
            flags.set_may_leave(false);
            ret.lower(&mut cx, &options, map_maybe_uninit!(storage.ret))?;
        } else {
            let storage =
                slice_to_storage_mut::<ReturnPointer<Params::Lower>>(storage).assume_init_ref();
            let params = Params::lift(cx.0, &options, &storage.args)?;
            let ret = closure(cx.as_context_mut(), params)?;
            let mut memory = MemoryMut::new(cx.as_context_mut(), &options);
            let ptr = validate_inbounds::<Return>(memory.as_slice_mut(), &storage.retptr)?;
            flags.set_may_leave(false);
            ret.store(&mut memory, ptr)?;
        }
    } else {
        let memory = Memory::new(cx.0, &options);
        if Return::flatten_count() <= MAX_FLAT_RESULTS {
            let storage = slice_to_storage_mut::<ReturnStack<ValRaw, Return::Lower>>(storage);
            let ptr =
                validate_inbounds::<Params>(memory.as_slice(), &storage.assume_init_ref().args)?;
            let params = Params::load(&memory, &memory.as_slice()[ptr..][..Params::SIZE32])?;
            let ret = closure(cx.as_context_mut(), params)?;
            flags.set_may_leave(false);
            ret.lower(&mut cx, &options, map_maybe_uninit!(storage.ret))?;
        } else {
            let storage = slice_to_storage_mut::<ReturnPointer<ValRaw>>(storage).assume_init_ref();
            let ptr = validate_inbounds::<Params>(memory.as_slice(), &storage.args)?;
            let params = Params::load(&memory, &memory.as_slice()[ptr..][..Params::SIZE32])?;
            let ret = closure(cx.as_context_mut(), params)?;
            let mut memory = MemoryMut::new(cx.as_context_mut(), &options);
            let ptr = validate_inbounds::<Return>(memory.as_slice_mut(), &storage.retptr)?;
            flags.set_may_leave(false);
            ret.store(&mut memory, ptr)?;
        }
    }

    flags.set_may_leave(true);

    return Ok(());
}

fn validate_inbounds<T: ComponentType>(memory: &[u8], ptr: &ValRaw) -> Result<usize> {
    // FIXME: needs memory64 support
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

unsafe fn handle_result(func: impl FnOnce() -> Result<()>) {
    match panic::catch_unwind(AssertUnwindSafe(func)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => crate::trap::raise(e),
        Err(e) => wasmtime_runtime::resume_panic(e),
    }
}

unsafe fn call_host_dynamic<T, F>(
    Types { params, results }: &Types,
    cx: *mut VMOpaqueContext,
    mut flags: InstanceFlags,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMCallerCheckedAnyfunc,
    string_encoding: StringEncoding,
    storage: &mut [ValRaw],
    closure: F,
) -> Result<()>
where
    F: FnOnce(StoreContextMut<'_, T>, &[Val], &mut [Val]) -> Result<()>,
{
    let cx = VMComponentContext::from_opaque(cx);
    let instance = (*cx).instance();
    let mut cx = StoreContextMut::from_raw((*instance).store());

    let options = Options::new(
        cx.0.id(),
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

    let param_abi = CanonicalAbiInfo::record(params.iter().map(|t| t.canonical_abi()));
    if let Some(param_count) = param_abi.flat_count(MAX_FLAT_PARAMS) {
        let iter = &mut storage.iter();
        args = params
            .iter()
            .map(|ty| Val::lift(ty, cx.0, &options, iter))
            .collect::<Result<Box<[_]>>>()?;
        ret_index = param_count;
    } else {
        let memory = Memory::new(cx.0, &options);
        let mut offset = validate_inbounds_dynamic(&param_abi, memory.as_slice(), &storage[0])?;
        args = params
            .iter()
            .map(|ty| {
                let abi = ty.canonical_abi();
                let size = usize::try_from(abi.size32).unwrap();
                Val::load(
                    ty,
                    &memory,
                    &memory.as_slice()[abi.next_field32_size(&mut offset)..][..size],
                )
            })
            .collect::<Result<Box<[_]>>>()?;
        ret_index = 1;
    };

    let mut result_vals = Vec::with_capacity(results.len());
    for _ in results.iter() {
        result_vals.push(Val::Bool(false));
    }
    closure(cx.as_context_mut(), &args, &mut result_vals)?;
    flags.set_may_leave(false);
    for (val, ty) in result_vals.iter().zip(results.iter()) {
        ty.check(val)?;
    }

    let result_abi = CanonicalAbiInfo::record(results.iter().map(|t| t.canonical_abi()));
    if result_abi.flat_count(MAX_FLAT_RESULTS).is_some() {
        let dst = mem::transmute::<&mut [ValRaw], &mut [MaybeUninit<ValRaw>]>(storage);
        let mut dst = dst.iter_mut();
        for val in result_vals.iter() {
            val.lower(&mut cx, &options, &mut dst)?;
        }
    } else {
        let ret_ptr = &storage[ret_index];
        let mut memory = MemoryMut::new(cx.as_context_mut(), &options);
        let mut ptr = validate_inbounds_dynamic(&result_abi, memory.as_slice_mut(), ret_ptr)?;
        for (val, ty) in result_vals.iter().zip(results.iter()) {
            let offset = ty.canonical_abi().next_field32_size(&mut ptr);
            val.store(&mut memory, offset)?;
        }
    }

    flags.set_may_leave(true);

    return Ok(());
}

fn validate_inbounds_dynamic(abi: &CanonicalAbiInfo, memory: &[u8], ptr: &ValRaw) -> Result<usize> {
    // FIXME: needs memory64 support
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

struct Types {
    params: Box<[Type]>,
    results: Box<[Type]>,
}

struct DynamicContext<F> {
    func: F,
    types: Types,
}

extern "C" fn dynamic_entrypoint<T, F>(
    cx: *mut VMOpaqueContext,
    data: *mut u8,
    flags: InstanceFlags,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMCallerCheckedAnyfunc,
    string_encoding: StringEncoding,
    storage: *mut ValRaw,
    storage_len: usize,
) where
    F: Fn(StoreContextMut<'_, T>, &[Val], &mut [Val]) -> Result<()> + Send + Sync + 'static,
{
    let data = data as *const DynamicContext<F>;
    unsafe {
        handle_result(|| {
            call_host_dynamic::<T, _>(
                &(*data).types,
                cx,
                flags,
                memory,
                realloc,
                string_encoding,
                std::slice::from_raw_parts_mut(storage, storage_len),
                |store, params, results| ((*data).func)(store, params, results),
            )
        })
    }
}
