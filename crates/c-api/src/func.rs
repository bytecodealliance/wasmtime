use crate::wasm_trap_t;
use crate::{
    wasm_extern_t, wasm_functype_t, wasm_store_t, wasm_val_t, wasm_val_vec_t, wasmtime_error_t,
    wasmtime_extern_t, wasmtime_val_t, wasmtime_val_union, CStoreContext, CStoreContextMut,
};
use anyhow::anyhow;
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::panic::{self, AssertUnwindSafe};
use std::ptr;
use std::str;
use wasmtime::{AsContextMut, Caller, Extern, Func, Trap, Val};

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_func_t {
    ext: wasm_extern_t,
}

wasmtime_c_api_macros::declare_ref!(wasm_func_t);

pub type wasm_func_callback_t = extern "C" fn(
    args: *const wasm_val_vec_t,
    results: *mut wasm_val_vec_t,
) -> Option<Box<wasm_trap_t>>;

pub type wasm_func_callback_with_env_t = extern "C" fn(
    env: *mut std::ffi::c_void,
    args: *const wasm_val_vec_t,
    results: *mut wasm_val_vec_t,
) -> Option<Box<wasm_trap_t>>;

impl wasm_func_t {
    pub(crate) fn try_from(e: &wasm_extern_t) -> Option<&wasm_func_t> {
        match &e.which {
            Extern::Func(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    pub(crate) fn func(&self) -> Func {
        match self.ext.which {
            Extern::Func(f) => f,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

unsafe fn create_function(
    store: &mut wasm_store_t,
    ty: &wasm_functype_t,
    func: impl Fn(*const wasm_val_vec_t, *mut wasm_val_vec_t) -> Option<Box<wasm_trap_t>>
        + Send
        + Sync
        + 'static,
) -> Box<wasm_func_t> {
    let ty = ty.ty().ty.clone();
    let func = Func::new(
        store.store.context_mut(),
        ty,
        move |_caller, params, results| {
            let params: wasm_val_vec_t = params
                .iter()
                .cloned()
                .map(|p| wasm_val_t::from_val(p))
                .collect::<Vec<_>>()
                .into();
            let mut out_results: wasm_val_vec_t = vec![wasm_val_t::default(); results.len()].into();
            let out = func(&params, &mut out_results);
            if let Some(trap) = out {
                return Err(trap.trap.clone());
            }

            let out_results = out_results.as_slice();
            for i in 0..results.len() {
                results[i] = out_results[i].val();
            }
            Ok(())
        },
    );
    Box::new(wasm_func_t {
        ext: wasm_extern_t {
            store: store.store.clone(),
            which: func.into(),
        },
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_new(
    store: &mut wasm_store_t,
    ty: &wasm_functype_t,
    callback: wasm_func_callback_t,
) -> Box<wasm_func_t> {
    create_function(store, ty, move |params, results| callback(params, results))
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_new_with_env(
    store: &mut wasm_store_t,
    ty: &wasm_functype_t,
    callback: wasm_func_callback_with_env_t,
    data: *mut c_void,
    finalizer: Option<extern "C" fn(arg1: *mut std::ffi::c_void)>,
) -> Box<wasm_func_t> {
    let finalizer = crate::ForeignData { data, finalizer };
    create_function(store, ty, move |params, results| {
        callback(finalizer.data, params, results)
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_call(
    func: &mut wasm_func_t,
    args: *const wasm_val_vec_t,
    results: *mut wasm_val_vec_t,
) -> *mut wasm_trap_t {
    let f = func.func();
    let results = (*results).as_uninit_slice();
    let args = (*args).as_slice();
    if results.len() != f.ty(func.ext.store.context()).results().len() {
        return Box::into_raw(Box::new(wasm_trap_t::new(
            anyhow!("wrong number of results provided").into(),
        )));
    }
    let params = args.iter().map(|i| i.val()).collect::<Vec<_>>();

    // We're calling arbitrary code here most of the time, and we in general
    // want to try to insulate callers against bugs in wasmtime/wasi/etc if we
    // can. As a result we catch panics here and transform them to traps to
    // allow the caller to have any insulation possible against Rust panics.
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        f.call(func.ext.store.context_mut(), &params)
    }));
    match result {
        Ok(Ok(out)) => {
            for (slot, val) in results.iter_mut().zip(out.into_vec().into_iter()) {
                crate::initialize(slot, wasm_val_t::from_val(val));
            }
            ptr::null_mut()
        }
        Ok(Err(trap)) => match trap.downcast::<Trap>() {
            Ok(trap) => Box::into_raw(Box::new(wasm_trap_t::new(trap))),
            Err(err) => Box::into_raw(Box::new(wasm_trap_t::new(err.into()))),
        },
        Err(panic) => {
            let trap = if let Some(msg) = panic.downcast_ref::<String>() {
                Trap::new(msg)
            } else if let Some(msg) = panic.downcast_ref::<&'static str>() {
                Trap::new(*msg)
            } else {
                Trap::new("rust panic happened")
            };
            let trap = Box::new(wasm_trap_t::new(trap));
            Box::into_raw(trap)
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_type(f: &wasm_func_t) -> Box<wasm_functype_t> {
    Box::new(wasm_functype_t::new(f.func().ty(f.ext.store.context())))
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_param_arity(f: &wasm_func_t) -> usize {
    f.func().ty(f.ext.store.context()).params().len()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_result_arity(f: &wasm_func_t) -> usize {
    f.func().ty(f.ext.store.context()).results().len()
}

#[no_mangle]
pub extern "C" fn wasm_func_as_extern(f: &mut wasm_func_t) -> &mut wasm_extern_t {
    &mut (*f).ext
}

#[no_mangle]
pub extern "C" fn wasm_func_as_extern_const(f: &wasm_func_t) -> &wasm_extern_t {
    &(*f).ext
}

#[repr(C)]
pub struct wasmtime_caller_t<'a> {
    caller: Caller<'a, crate::StoreData>,
}

pub type wasmtime_func_callback_t = extern "C" fn(
    *mut c_void,
    *mut wasmtime_caller_t,
    *const wasmtime_val_t,
    usize,
    *mut wasmtime_val_t,
    usize,
) -> Option<Box<wasm_trap_t>>;

#[no_mangle]
pub unsafe extern "C" fn wasmtime_func_new(
    store: CStoreContextMut<'_>,
    ty: &wasm_functype_t,
    callback: wasmtime_func_callback_t,
    data: *mut c_void,
    finalizer: Option<extern "C" fn(*mut std::ffi::c_void)>,
    func: &mut Func,
) {
    let ty = ty.ty().ty.clone();
    let cb = c_callback_to_rust_fn(callback, data, finalizer);
    let f = Func::new(store, ty, cb);
    *func = f;
}

pub(crate) unsafe fn c_callback_to_rust_fn(
    callback: wasmtime_func_callback_t,
    data: *mut c_void,
    finalizer: Option<extern "C" fn(*mut std::ffi::c_void)>,
) -> impl Fn(Caller<'_, crate::StoreData>, &[Val], &mut [Val]) -> Result<(), Trap> {
    let foreign = crate::ForeignData { data, finalizer };
    move |caller, params, results| {
        let params = params
            .iter()
            .cloned()
            .map(|p| wasmtime_val_t::from_val(p))
            .collect::<Vec<_>>();
        let mut out_results = (0..results.len())
            .map(|_| wasmtime_val_t {
                kind: crate::WASMTIME_I32,
                of: wasmtime_val_union { i32: 0 },
            })
            .collect::<Vec<_>>();
        let mut caller = wasmtime_caller_t { caller };
        let out = callback(
            foreign.data,
            &mut caller,
            params.as_ptr(),
            params.len(),
            out_results.as_mut_ptr(),
            out_results.len(),
        );
        if let Some(trap) = out {
            return Err(trap.trap);
        }

        for (i, result) in out_results.iter().enumerate() {
            results[i] = unsafe { result.to_val() };
        }
        Ok(())
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_func_call(
    store: CStoreContextMut<'_>,
    func: &Func,
    args: *const wasmtime_val_t,
    nargs: usize,
    results: *mut MaybeUninit<wasmtime_val_t>,
    nresults: usize,
    trap_ret: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    if nresults != func.ty(&store).results().len() {
        return Some(Box::new(wasmtime_error_t::from(anyhow!(
            "wrong number of results provided"
        ))));
    }
    let params = crate::slice_from_raw_parts(args, nargs)
        .iter()
        .map(|i| i.to_val())
        .collect::<Vec<_>>();

    // We're calling arbitrary code here most of the time, and we in general
    // want to try to insulate callers against bugs in wasmtime/wasi/etc if we
    // can. As a result we catch panics here and transform them to traps to
    // allow the caller to have any insulation possible against Rust panics.
    let result = panic::catch_unwind(AssertUnwindSafe(|| func.call(store, &params)));
    match result {
        Ok(Ok(out)) => {
            let results = crate::slice_from_raw_parts_mut(results, nresults);
            for (slot, val) in results.iter_mut().zip(out.into_vec().into_iter()) {
                crate::initialize(slot, wasmtime_val_t::from_val(val));
            }
            None
        }
        Ok(Err(trap)) => match trap.downcast::<Trap>() {
            Ok(trap) => {
                *trap_ret = Box::into_raw(Box::new(wasm_trap_t::new(trap)));
                None
            }
            Err(err) => Some(Box::new(wasmtime_error_t::from(err))),
        },
        Err(panic) => {
            let trap = if let Some(msg) = panic.downcast_ref::<String>() {
                Trap::new(msg)
            } else if let Some(msg) = panic.downcast_ref::<&'static str>() {
                Trap::new(*msg)
            } else {
                Trap::new("rust panic happened")
            };
            *trap_ret = Box::into_raw(Box::new(wasm_trap_t::new(trap)));
            None
        }
    }
}

#[no_mangle]
pub extern "C" fn wasmtime_func_type(
    store: CStoreContext<'_>,
    func: &Func,
) -> Box<wasm_functype_t> {
    Box::new(wasm_functype_t::new(func.ty(store)))
}

#[no_mangle]
pub extern "C" fn wasmtime_caller_context<'a>(
    caller: &'a mut wasmtime_caller_t,
) -> CStoreContextMut<'a> {
    caller.caller.as_context_mut()
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_caller_export_get(
    caller: &mut wasmtime_caller_t,
    name: *const u8,
    name_len: usize,
    item: &mut MaybeUninit<wasmtime_extern_t>,
) -> bool {
    let name = match str::from_utf8(crate::slice_from_raw_parts(name, name_len)) {
        Ok(name) => name,
        Err(_) => return false,
    };
    let which = match caller.caller.get_export(name) {
        Some(item) => item,
        None => return false,
    };
    crate::initialize(item, which.into());
    true
}
