use crate::{
    bad_utf8, handle_result, wasm_engine_t, wasm_trap_t, wasmtime_error_t, wasmtime_extern_t,
    wasmtime_module_t, CStoreContextMut,
};
use std::mem::MaybeUninit;
use std::str;
use wasmtime::{Func, Instance, Linker};

#[repr(C)]
pub struct wasmtime_linker_t {
    linker: Linker<crate::StoreData>,
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_new(engine: &wasm_engine_t) -> Box<wasmtime_linker_t> {
    Box::new(wasmtime_linker_t {
        linker: Linker::new(&engine.engine),
    })
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_allow_shadowing(
    linker: &mut wasmtime_linker_t,
    allow_shadowing: bool,
) {
    linker.linker.allow_shadowing(allow_shadowing);
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_delete(_linker: Box<wasmtime_linker_t>) {}

macro_rules! to_str {
    ($ptr:expr, $len:expr) => {
        match str::from_utf8(crate::slice_from_raw_parts($ptr, $len)) {
            Ok(s) => s,
            Err(_) => return bad_utf8(),
        }
    };
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_linker_define(
    linker: &mut wasmtime_linker_t,
    module: *const u8,
    module_len: usize,
    name: *const u8,
    name_len: usize,
    item: &wasmtime_extern_t,
) -> Option<Box<wasmtime_error_t>> {
    let linker = &mut linker.linker;
    let module = to_str!(module, module_len);
    let name = to_str!(name, name_len);
    let item = item.to_extern();
    handle_result(linker.define(module, name, item), |_linker| ())
}

#[cfg(feature = "wasi")]
#[no_mangle]
pub extern "C" fn wasmtime_linker_define_wasi(
    linker: &mut wasmtime_linker_t,
) -> Option<Box<wasmtime_error_t>> {
    handle_result(
        wasmtime_wasi::add_to_linker(&mut linker.linker, |cx| cx.wasi.as_mut().unwrap()),
        |_linker| (),
    )
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_linker_define_instance(
    linker: &mut wasmtime_linker_t,
    store: CStoreContextMut<'_>,
    name: *const u8,
    name_len: usize,
    instance: &Instance,
) -> Option<Box<wasmtime_error_t>> {
    let linker = &mut linker.linker;
    let name = to_str!(name, name_len);
    handle_result(linker.instance(store, name, *instance), |_linker| ())
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_instantiate(
    linker: &wasmtime_linker_t,
    store: CStoreContextMut<'_>,
    module: &wasmtime_module_t,
    instance_ptr: &mut Instance,
    trap_ptr: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    let result = linker.linker.instantiate(store, &module.module);
    super::instance::handle_instantiate(result, instance_ptr, trap_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_linker_module(
    linker: &mut wasmtime_linker_t,
    store: CStoreContextMut<'_>,
    name: *const u8,
    name_len: usize,
    module: &wasmtime_module_t,
) -> Option<Box<wasmtime_error_t>> {
    let linker = &mut linker.linker;
    let name = to_str!(name, name_len);
    handle_result(linker.module(store, name, &module.module), |_linker| ())
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_linker_get_default(
    linker: &wasmtime_linker_t,
    store: CStoreContextMut<'_>,
    name: *const u8,
    name_len: usize,
    func: &mut Func,
) -> Option<Box<wasmtime_error_t>> {
    let linker = &linker.linker;
    let name = to_str!(name, name_len);
    handle_result(linker.get_default(store, name), |f| *func = f)
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_linker_get(
    linker: &wasmtime_linker_t,
    store: CStoreContextMut<'_>,
    module: *const u8,
    module_len: usize,
    name: *const u8,
    name_len: usize,
    item_ptr: &mut MaybeUninit<wasmtime_extern_t>,
) -> bool {
    let linker = &linker.linker;
    let module = match str::from_utf8(crate::slice_from_raw_parts(module, module_len)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let name = if name.is_null() {
        None
    } else {
        match str::from_utf8(crate::slice_from_raw_parts(name, name_len)) {
            Ok(s) => Some(s),
            Err(_) => return false,
        }
    };
    match linker.get(store, module, name) {
        Some(which) => {
            crate::initialize(item_ptr, which.into());
            true
        }
        None => false,
    }
}
