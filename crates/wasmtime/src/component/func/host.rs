use crate::component::func::{MAX_STACK_PARAMS, MAX_STACK_RESULTS};
use crate::component::{ComponentParams, ComponentValue, Memory, MemoryMut, Op, Options};
use crate::{AsContextMut, StoreContextMut, ValRaw};
use anyhow::{bail, Context, Result};
use std::any::Any;
use std::mem::MaybeUninit;
use std::panic::{self, AssertUnwindSafe};
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::component::{ComponentTypes, FuncTypeIndex, StringEncoding};
use wasmtime_runtime::component::{VMComponentContext, VMLowering, VMLoweringCallee};
use wasmtime_runtime::{VMCallerCheckedAnyfunc, VMMemoryDefinition, VMOpaqueContext};

/// Trait representing host-defined functions that can be imported into a wasm
/// component.
///
/// For more information see the
/// [`Linker::func_wrap`](crate::component::Linker::func_wrap) documentation.
pub trait IntoComponentFunc<T, Params, Return> {
    /// Host entrypoint from a cranelift-generated trampoline.
    ///
    /// This function has type `VMLoweringCallee` and delegates to the shared
    /// `call_host` function below.
    #[doc(hidden)]
    extern "C" fn entrypoint(
        cx: *mut VMOpaqueContext,
        data: *mut u8,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMCallerCheckedAnyfunc,
        string_encoding: StringEncoding,
        storage: *mut ValRaw,
        storage_len: usize,
    );

    #[doc(hidden)]
    fn into_host_func(self) -> Arc<HostFunc>;
}

pub struct HostFunc {
    entrypoint: VMLoweringCallee,
    typecheck: fn(FuncTypeIndex, &ComponentTypes) -> Result<()>,
    func: Box<dyn Any + Send + Sync>,
}

impl HostFunc {
    fn new<F, P, R>(func: F, entrypoint: VMLoweringCallee) -> Arc<HostFunc>
    where
        F: Send + Sync + 'static,
        P: ComponentParams,
        R: ComponentValue,
    {
        Arc::new(HostFunc {
            entrypoint,
            typecheck: typecheck::<P, R>,
            func: Box::new(func),
        })
    }

    pub fn typecheck(&self, ty: FuncTypeIndex, types: &ComponentTypes) -> Result<()> {
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

fn typecheck<P, R>(ty: FuncTypeIndex, types: &ComponentTypes) -> Result<()>
where
    P: ComponentParams,
    R: ComponentValue,
{
    let ty = &types[ty];
    P::typecheck(&ty.params, types, Op::Lift).context("type mismatch with parameters")?;
    R::typecheck(&ty.result, types, Op::Lower).context("type mismatch with result")?;
    Ok(())
}

/// The "meat" of calling a host function from wasm.
///
/// This function is delegated to from implementations of `IntoComponentFunc`
/// generated in the macro below. Most of the arguments from the `entrypoint`
/// are forwarded here except for the `data` pointer which is encapsulated in
/// the `closure` argument here.
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
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMCallerCheckedAnyfunc,
    string_encoding: StringEncoding,
    storage: &mut [ValRaw],
    closure: F,
) -> Result<()>
where
    Params: ComponentValue,
    Return: ComponentValue,
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
    let may_leave = (*instance).may_leave();
    let may_enter = (*instance).may_enter();
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
    if !*may_leave {
        bail!("cannot leave component instance");
    }

    // While we're lifting and lowering this instance cannot be reentered, so
    // unset the flag here. This is also reset back to `true` on exit.
    let _reset_may_enter = unset_and_reset_on_drop(may_enter);

    // There's a 2x2 matrix of whether parameters and results are stored on the
    // stack or on the heap. Each of the 4 branches here have a different
    // representation of the storage of arguments/returns which is represented
    // by the type parameter that we pass to `cast_storage`.
    //
    // Also note that while four branches are listed here only one is taken for
    // any particular `Params` and `Return` combination. This should be
    // trivially DCE'd by LLVM. Perhaps one day with enough const programming in
    // Rust we can make monomorphizations of this function codegen only one
    // branch, but today is not that day.
    let reset_may_leave;
    if Params::flatten_count() <= MAX_STACK_PARAMS {
        if Return::flatten_count() <= MAX_STACK_RESULTS {
            let storage = cast_storage::<ReturnStack<Params::Lower, Return::Lower>>(storage);
            let params = Params::lift(cx.0, &options, &storage.assume_init_ref().args)?;
            let ret = closure(cx.as_context_mut(), params)?;
            reset_may_leave = unset_and_reset_on_drop(may_leave);
            ret.lower(&mut cx, &options, map_maybe_uninit!(storage.ret))?;
        } else {
            let storage = cast_storage::<ReturnPointer<Params::Lower>>(storage).assume_init_ref();
            let params = Params::lift(cx.0, &options, &storage.args)?;
            let ret = closure(cx.as_context_mut(), params)?;
            let mut memory = MemoryMut::new(cx.as_context_mut(), &options);
            let ptr = validate_inbounds::<Return>(memory.as_slice_mut(), &storage.retptr)?;
            reset_may_leave = unset_and_reset_on_drop(may_leave);
            ret.store(&mut memory, ptr)?;
        }
    } else {
        let memory = Memory::new(cx.0, &options);
        if Return::flatten_count() <= MAX_STACK_RESULTS {
            let storage = cast_storage::<ReturnStack<ValRaw, Return::Lower>>(storage);
            let ptr =
                validate_inbounds::<Params>(memory.as_slice(), &storage.assume_init_ref().args)?;
            let params = Params::load(&memory, &memory.as_slice()[ptr..][..Params::size()])?;
            let ret = closure(cx.as_context_mut(), params)?;
            reset_may_leave = unset_and_reset_on_drop(may_leave);
            ret.lower(&mut cx, &options, map_maybe_uninit!(storage.ret))?;
        } else {
            let storage = cast_storage::<ReturnPointer<ValRaw>>(storage).assume_init_ref();
            let ptr = validate_inbounds::<Params>(memory.as_slice(), &storage.args)?;
            let params = Params::load(&memory, &memory.as_slice()[ptr..][..Params::size()])?;
            let ret = closure(cx.as_context_mut(), params)?;
            let mut memory = MemoryMut::new(cx.as_context_mut(), &options);
            let ptr = validate_inbounds::<Return>(memory.as_slice_mut(), &storage.retptr)?;
            reset_may_leave = unset_and_reset_on_drop(may_leave);
            ret.store(&mut memory, ptr)?;
        }
    }

    // TODO: need to call `post-return` before this `drop`
    drop(reset_may_leave);

    return Ok(());

    unsafe fn unset_and_reset_on_drop(slot: *mut bool) -> impl Drop {
        debug_assert!(*slot);
        *slot = false;
        return Reset(slot);

        struct Reset(*mut bool);

        impl Drop for Reset {
            fn drop(&mut self) {
                unsafe {
                    (*self.0) = true;
                }
            }
        }
    }
}

fn validate_inbounds<T: ComponentValue>(memory: &[u8], ptr: &ValRaw) -> Result<usize> {
    // FIXME: needs memory64 support
    let ptr = usize::try_from(ptr.get_u32())?;
    let end = match ptr.checked_add(T::size()) {
        Some(n) => n,
        None => bail!("return pointer size overflow"),
    };
    if end > memory.len() {
        bail!("return pointer out of bounds")
    }
    Ok(ptr)
}

unsafe fn cast_storage<T>(storage: &mut [ValRaw]) -> &mut MaybeUninit<T> {
    // Assertions that LLVM can easily optimize away but are sanity checks here
    assert!(std::mem::size_of::<T>() % std::mem::size_of::<ValRaw>() == 0);
    assert!(std::mem::align_of::<T>() == std::mem::align_of::<ValRaw>());
    assert!(std::mem::align_of_val(storage) == std::mem::align_of::<T>());

    // This is an actual runtime assertion which if performance calls for we may
    // need to relax to a debug assertion. This notably tries to ensure that we
    // stay within the bounds of the number of actual values given rather than
    // reading past the end of an array. This shouldn't actually trip unless
    // there's a bug in Wasmtime though.
    assert!(std::mem::size_of_val(storage) >= std::mem::size_of::<T>());

    &mut *storage.as_mut_ptr().cast()
}

unsafe fn handle_result(func: impl FnOnce() -> Result<()>) {
    match panic::catch_unwind(AssertUnwindSafe(func)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => wasmtime_runtime::raise_user_trap(e),
        Err(e) => wasmtime_runtime::resume_panic(e),
    }
}

macro_rules! impl_into_component_func {
    ($num:tt $($args:ident)*) => {
        // Implement for functions without a leading `StoreContextMut` parameter
        #[allow(non_snake_case)]
        impl<T, F, $($args,)* R> IntoComponentFunc<T, ($($args,)*), R> for F
        where
            F: Fn($($args),*) -> Result<R> + Send + Sync + 'static,
            ($($args,)*): ComponentParams + ComponentValue,
            R: ComponentValue,
        {
            extern "C" fn entrypoint(
                cx: *mut VMOpaqueContext,
                data: *mut u8,
                memory: *mut VMMemoryDefinition,
                realloc: *mut VMCallerCheckedAnyfunc,
                string_encoding: StringEncoding,
                storage: *mut ValRaw,
                storage_len: usize,
            ) {
                let data = data as *const Self;
                unsafe {
                    handle_result(|| call_host::<T, _, _, _>(
                        cx,
                        memory,
                        realloc,
                        string_encoding,
                        std::slice::from_raw_parts_mut(storage, storage_len),
                        |_, ($($args,)*)| (*data)($($args),*),
                    ))
                }
            }

            fn into_host_func(self) -> Arc<HostFunc> {
                let entrypoint = <Self as IntoComponentFunc<T, ($($args,)*), R>>::entrypoint;
                HostFunc::new::<_, ($($args,)*), R>(self, entrypoint)
            }
        }

        // Implement for functions with a leading `StoreContextMut` parameter
        #[allow(non_snake_case)]
        impl<T, F, $($args,)* R> IntoComponentFunc<T, (StoreContextMut<'_, T>, $($args,)*), R> for F
        where
            F: Fn(StoreContextMut<'_, T>, $($args),*) -> Result<R> + Send + Sync + 'static,
            ($($args,)*): ComponentParams + ComponentValue,
            R: ComponentValue,
        {
            extern "C" fn entrypoint(
                cx: *mut VMOpaqueContext,
                data: *mut u8,
                memory: *mut VMMemoryDefinition,
                realloc: *mut VMCallerCheckedAnyfunc,
                string_encoding: StringEncoding,
                storage: *mut ValRaw,
                storage_len: usize,
            ) {
                let data = data as *const Self;
                unsafe {
                    handle_result(|| call_host::<T, _, _, _>(
                        cx,
                        memory,
                        realloc,
                        string_encoding,
                        std::slice::from_raw_parts_mut(storage, storage_len),
                        |store, ($($args,)*)| (*data)(store, $($args),*),
                    ))
                }
            }

            fn into_host_func(self) -> Arc<HostFunc> {
                let entrypoint = <Self as IntoComponentFunc<T, (StoreContextMut<'_, T>, $($args,)*), R>>::entrypoint;
                HostFunc::new::<_, ($($args,)*), R>(self, entrypoint)
            }
        }
    }
}

for_each_function_signature!(impl_into_component_func);
