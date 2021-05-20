use crate::{
    handle_result, wasm_extern_t, wasm_globaltype_t, wasm_store_t, wasm_val_t, wasmtime_error_t,
    wasmtime_val_t, CStoreContext, CStoreContextMut,
};
use std::mem::MaybeUninit;
use wasmtime::{Extern, Global};

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_global_t {
    ext: wasm_extern_t,
}

wasmtime_c_api_macros::declare_ref!(wasm_global_t);

impl wasm_global_t {
    pub(crate) fn try_from(e: &wasm_extern_t) -> Option<&wasm_global_t> {
        match &e.which {
            Extern::Global(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    fn global(&self) -> Global {
        match self.ext.which {
            Extern::Global(g) => g,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_new(
    store: &mut wasm_store_t,
    gt: &wasm_globaltype_t,
    val: &wasm_val_t,
) -> Option<Box<wasm_global_t>> {
    match Global::new(store.store.context_mut(), gt.ty().ty.clone(), val.val()) {
        Ok(global) => Some(Box::new(wasm_global_t {
            ext: wasm_extern_t {
                store: store.store.clone(),
                which: global.into(),
            },
        })),
        Err(_) => None,
    }
}

#[no_mangle]
pub extern "C" fn wasm_global_as_extern(g: &mut wasm_global_t) -> &mut wasm_extern_t {
    &mut g.ext
}

#[no_mangle]
pub extern "C" fn wasm_global_as_extern_const(g: &wasm_global_t) -> &wasm_extern_t {
    &g.ext
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_type(g: &wasm_global_t) -> Box<wasm_globaltype_t> {
    let globaltype = g.global().ty(&g.ext.store.context());
    Box::new(wasm_globaltype_t::new(globaltype))
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_get(g: &mut wasm_global_t, out: &mut MaybeUninit<wasm_val_t>) {
    let global = g.global();
    crate::initialize(
        out,
        wasm_val_t::from_val(global.get(g.ext.store.context_mut())),
    );
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_set(g: &mut wasm_global_t, val: &wasm_val_t) {
    let global = g.global();
    drop(global.set(g.ext.store.context_mut(), val.val()));
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_global_new(
    store: CStoreContextMut<'_>,
    gt: &wasm_globaltype_t,
    val: &wasmtime_val_t,
    ret: &mut Global,
) -> Option<Box<wasmtime_error_t>> {
    let global = Global::new(store, gt.ty().ty.clone(), val.to_val());
    handle_result(global, |global| {
        *ret = global;
    })
}

#[no_mangle]
pub extern "C" fn wasmtime_global_type(
    store: CStoreContext<'_>,
    global: &Global,
) -> Box<wasm_globaltype_t> {
    Box::new(wasm_globaltype_t::new(global.ty(store)))
}

#[no_mangle]
pub extern "C" fn wasmtime_global_get(
    store: CStoreContextMut<'_>,
    global: &Global,
    val: &mut MaybeUninit<wasmtime_val_t>,
) {
    crate::initialize(val, wasmtime_val_t::from_val(global.get(store)))
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_global_set(
    store: CStoreContextMut<'_>,
    global: &Global,
    val: &wasmtime_val_t,
) -> Option<Box<wasmtime_error_t>> {
    handle_result(global.set(store, val.to_val()), |()| {})
}
