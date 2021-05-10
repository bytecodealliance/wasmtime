use crate::wasm_trap_t;
use crate::{wasm_extern_t, wasm_functype_t, wasm_store_t, wasm_val_t, wasm_val_vec_t};
use anyhow::anyhow;
use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};
use std::ptr;
use std::str;
use wasmtime::{Extern, Func, Trap};

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_func_t {
    ext: wasm_extern_t,
}

wasmtime_c_api_macros::declare_ref!(wasm_func_t);

// #[repr(C)]
// pub struct wasmtime_caller_t<'a> {
//     caller: Caller<'a>,
// }

pub type wasm_func_callback_t = extern "C" fn(
    args: *const wasm_val_vec_t,
    results: *mut wasm_val_vec_t,
) -> Option<Box<wasm_trap_t>>;

pub type wasm_func_callback_with_env_t = extern "C" fn(
    env: *mut std::ffi::c_void,
    args: *const wasm_val_vec_t,
    results: *mut wasm_val_vec_t,
) -> Option<Box<wasm_trap_t>>;

// pub type wasmtime_func_callback_t = extern "C" fn(
//     caller: *const wasmtime_caller_t,
//     args: *const wasm_val_vec_t,
//     results: *mut wasm_val_vec_t,
// ) -> Option<Box<wasm_trap_t>>;

// pub type wasmtime_func_callback_with_env_t = extern "C" fn(
//     caller: *const wasmtime_caller_t,
//     env: *mut std::ffi::c_void,
//     args: *const wasm_val_vec_t,
//     results: *mut wasm_val_vec_t,
// ) -> Option<Box<wasm_trap_t>>;

struct Finalizer {
    env: *mut c_void,
    finalizer: Option<extern "C" fn(*mut c_void)>,
}

unsafe impl Send for Finalizer {}
unsafe impl Sync for Finalizer {}

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

// #[no_mangle]
// pub unsafe extern "C" fn wasmtime_func_new(
//     store: &wasm_store_t,
//     ty: &wasm_functype_t,
//     callback: wasmtime_func_callback_t,
// ) -> Box<wasm_func_t> {
//     create_function(store, ty, move |params, results| {
//         callback(&wasmtime_caller_t { caller }, params, results)
//     })
// }

#[no_mangle]
pub unsafe extern "C" fn wasm_func_new_with_env(
    store: &mut wasm_store_t,
    ty: &wasm_functype_t,
    callback: wasm_func_callback_with_env_t,
    env: *mut c_void,
    finalizer: Option<extern "C" fn(arg1: *mut std::ffi::c_void)>,
) -> Box<wasm_func_t> {
    let finalizer = Finalizer { env, finalizer };
    create_function(store, ty, move |params, results| {
        callback(finalizer.env, params, results)
    })
}

// #[no_mangle]
// pub extern "C" fn wasmtime_func_new_with_env(
//     store: &wasm_store_t,
//     ty: &wasm_functype_t,
//     callback: wasmtime_func_callback_with_env_t,
//     env: *mut c_void,
//     finalizer: Option<extern "C" fn(*mut c_void)>,
// ) -> Box<wasm_func_t> {
//     let finalizer = Finalizer { env, finalizer };
//     create_function(store, ty, move |caller, params, results| {
//         callback(
//             &wasmtime_caller_t { caller },
//             finalizer.env,
//             params,
//             results,
//         )
//     })
// }

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

// #[no_mangle]
// pub extern "C" fn wasmtime_caller_export_get(
//     caller: &wasmtime_caller_t,
//     name: &wasm_name_t,
// ) -> Option<Box<wasm_extern_t>> {
//     let name = str::from_utf8(name.as_slice()).ok()?;
//     let which = caller.caller.get_export(name)?;
//     Some(Box::new(wasm_extern_t { which }))
// }

// #[no_mangle]
// pub extern "C" fn wasmtime_func_as_funcref(
//     func: &wasm_func_t,
//     funcrefp: &mut MaybeUninit<wasm_val_t>,
// ) {
//     let funcref = wasm_val_t::from_val(Val::FuncRef(Some(func.func().clone())));
//     crate::initialize(funcrefp, funcref);
// }

// #[no_mangle]
// pub extern "C" fn wasmtime_funcref_as_func(val: &wasm_val_t) -> Option<Box<wasm_func_t>> {
//     if let Val::FuncRef(Some(f)) = val.val() {
//         Some(Box::new(f.into()))
//     } else {
//         None
//     }
// }
