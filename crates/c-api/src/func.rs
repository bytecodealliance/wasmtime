use crate::{wasm_extern_t, wasm_functype_t, wasm_store_t, wasm_val_t};
use crate::{wasm_name_t, wasm_trap_t, wasmtime_error_t, ExternHost};
use anyhow::anyhow;
use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};
use std::ptr;
use std::str;
use wasmtime::{Caller, Extern, Func, HostRef, Trap};

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_func_t {
    ext: wasm_extern_t,
}

wasmtime_c_api_macros::declare_ref!(wasm_func_t);

#[repr(C)]
pub struct wasmtime_caller_t<'a> {
    caller: Caller<'a>,
}

pub type wasm_func_callback_t =
    extern "C" fn(args: *const wasm_val_t, results: *mut wasm_val_t) -> Option<Box<wasm_trap_t>>;

pub type wasm_func_callback_with_env_t = extern "C" fn(
    env: *mut std::ffi::c_void,
    args: *const wasm_val_t,
    results: *mut wasm_val_t,
) -> Option<Box<wasm_trap_t>>;

pub type wasmtime_func_callback_t = extern "C" fn(
    caller: *const wasmtime_caller_t,
    args: *const wasm_val_t,
    results: *mut wasm_val_t,
) -> Option<Box<wasm_trap_t>>;

pub type wasmtime_func_callback_with_env_t = extern "C" fn(
    caller: *const wasmtime_caller_t,
    env: *mut std::ffi::c_void,
    args: *const wasm_val_t,
    results: *mut wasm_val_t,
) -> Option<Box<wasm_trap_t>>;

struct Finalizer {
    env: *mut c_void,
    finalizer: Option<extern "C" fn(*mut c_void)>,
}

impl Drop for Finalizer {
    fn drop(&mut self) {
        if let Some(f) = self.finalizer {
            f(self.env);
        }
    }
}

impl wasm_func_t {
    pub(crate) fn try_from(e: &wasm_extern_t) -> Option<&wasm_func_t> {
        match &e.which {
            ExternHost::Func(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    fn func(&self) -> &HostRef<Func> {
        match &self.ext.which {
            ExternHost::Func(f) => f,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    fn anyref(&self) -> wasmtime::AnyRef {
        self.func().anyref()
    }
}

fn create_function(
    store: &wasm_store_t,
    ty: &wasm_functype_t,
    func: impl Fn(Caller<'_>, *const wasm_val_t, *mut wasm_val_t) -> Option<Box<wasm_trap_t>> + 'static,
) -> Box<wasm_func_t> {
    let store = &store.store.borrow();
    let ty = ty.ty().ty.clone();
    let func = Func::new(store, ty, move |caller, params, results| {
        let params = params
            .iter()
            .map(|p| wasm_val_t::from_val(p))
            .collect::<Vec<_>>();
        let mut out_results = vec![wasm_val_t::default(); results.len()];
        let out = func(caller, params.as_ptr(), out_results.as_mut_ptr());
        if let Some(trap) = out {
            return Err(trap.trap.borrow().clone());
        }
        for i in 0..results.len() {
            results[i] = out_results[i].val();
        }
        Ok(())
    });
    Box::new(wasm_func_t {
        ext: wasm_extern_t {
            which: ExternHost::Func(HostRef::new(func)),
        },
    })
}

#[no_mangle]
pub extern "C" fn wasm_func_new(
    store: &wasm_store_t,
    ty: &wasm_functype_t,
    callback: wasm_func_callback_t,
) -> Box<wasm_func_t> {
    create_function(store, ty, move |_caller, params, results| {
        callback(params, results)
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_func_new(
    store: &wasm_store_t,
    ty: &wasm_functype_t,
    callback: wasmtime_func_callback_t,
) -> Box<wasm_func_t> {
    create_function(store, ty, move |caller, params, results| {
        callback(&wasmtime_caller_t { caller }, params, results)
    })
}

#[no_mangle]
pub extern "C" fn wasm_func_new_with_env(
    store: &wasm_store_t,
    ty: &wasm_functype_t,
    callback: wasm_func_callback_with_env_t,
    env: *mut c_void,
    finalizer: Option<extern "C" fn(arg1: *mut std::ffi::c_void)>,
) -> Box<wasm_func_t> {
    let finalizer = Finalizer { env, finalizer };
    create_function(store, ty, move |_caller, params, results| {
        callback(finalizer.env, params, results)
    })
}

#[no_mangle]
pub extern "C" fn wasmtime_func_new_with_env(
    store: &wasm_store_t,
    ty: &wasm_functype_t,
    callback: wasmtime_func_callback_with_env_t,
    env: *mut c_void,
    finalizer: Option<extern "C" fn(*mut c_void)>,
) -> Box<wasm_func_t> {
    let finalizer = Finalizer { env, finalizer };
    create_function(store, ty, move |caller, params, results| {
        callback(
            &wasmtime_caller_t { caller },
            finalizer.env,
            params,
            results,
        )
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_call(
    wasm_func: &wasm_func_t,
    args: *const wasm_val_t,
    results: *mut wasm_val_t,
) -> *mut wasm_trap_t {
    let func = wasm_func.func().borrow();
    let mut trap = ptr::null_mut();
    let error = wasmtime_func_call(
        wasm_func,
        args,
        func.param_arity(),
        results,
        func.result_arity(),
        &mut trap,
    );
    match error {
        Some(err) => Box::into_raw(err.to_trap()),
        None => trap,
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_func_call(
    func: &wasm_func_t,
    args: *const wasm_val_t,
    num_args: usize,
    results: *mut wasm_val_t,
    num_results: usize,
    trap_ptr: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    _wasmtime_func_call(
        func,
        std::slice::from_raw_parts(args, num_args),
        std::slice::from_raw_parts_mut(results, num_results),
        trap_ptr,
    )
}

fn _wasmtime_func_call(
    func: &wasm_func_t,
    args: &[wasm_val_t],
    results: &mut [wasm_val_t],
    trap_ptr: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    let func = func.func().borrow();
    if results.len() != func.result_arity() {
        return Some(Box::new(anyhow!("wrong number of results provided").into()));
    }
    let params = args.iter().map(|i| i.val()).collect::<Vec<_>>();

    // We're calling arbitrary code here most of the time, and we in general
    // want to try to insulate callers against bugs in wasmtime/wasi/etc if we
    // can. As a result we catch panics here and transform them to traps to
    // allow the caller to have any insulation possible against Rust panics.
    let result = panic::catch_unwind(AssertUnwindSafe(|| func.call(&params)));
    match result {
        Ok(Ok(out)) => {
            for (slot, val) in results.iter_mut().zip(out.iter()) {
                *slot = wasm_val_t::from_val(val);
            }
            None
        }
        Ok(Err(trap)) => match trap.downcast::<Trap>() {
            Ok(trap) => {
                *trap_ptr = Box::into_raw(Box::new(wasm_trap_t::new(trap)));
                None
            }
            Err(err) => Some(Box::new(err.into())),
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
            *trap_ptr = Box::into_raw(trap);
            None
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_func_type(f: &wasm_func_t) -> Box<wasm_functype_t> {
    Box::new(wasm_functype_t::new(f.func().borrow().ty()))
}

#[no_mangle]
pub extern "C" fn wasm_func_param_arity(f: &wasm_func_t) -> usize {
    f.func().borrow().param_arity()
}

#[no_mangle]
pub extern "C" fn wasm_func_result_arity(f: &wasm_func_t) -> usize {
    f.func().borrow().result_arity()
}

#[no_mangle]
pub extern "C" fn wasm_func_as_extern(f: &mut wasm_func_t) -> &mut wasm_extern_t {
    &mut (*f).ext
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_caller_export_get(
    caller: &wasmtime_caller_t,
    name: &wasm_name_t,
) -> Option<Box<wasm_extern_t>> {
    let name = str::from_utf8(name.as_slice()).ok()?;
    let export = caller.caller.get_export(name)?;
    let which = match export {
        Extern::Func(f) => ExternHost::Func(HostRef::new(f)),
        Extern::Global(g) => ExternHost::Global(HostRef::new(g)),
        Extern::Memory(m) => ExternHost::Memory(HostRef::new(m)),
        Extern::Table(t) => ExternHost::Table(HostRef::new(t)),
    };
    Some(Box::new(wasm_extern_t { which }))
}
