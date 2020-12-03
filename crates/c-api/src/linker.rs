use crate::{bad_utf8, handle_result, wasmtime_error_t};
use crate::{wasm_extern_t, wasm_store_t};
use crate::{wasm_func_t, wasm_instance_t, wasm_module_t, wasm_name_t, wasm_trap_t};
use std::str;
use wasmtime::Linker;

#[repr(C)]
pub struct wasmtime_linker_t {
    linker: Linker,
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_new(store: &wasm_store_t) -> Box<wasmtime_linker_t> {
    Box::new(wasmtime_linker_t {
        linker: Linker::new(&store.store),
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

#[no_mangle]
pub extern "C" fn wasmtime_linker_define(
    linker: &mut wasmtime_linker_t,
    module: &wasm_name_t,
    name: &wasm_name_t,
    item: &wasm_extern_t,
) -> Option<Box<wasmtime_error_t>> {
    let linker = &mut linker.linker;
    let module = match str::from_utf8(module.as_slice()) {
        Ok(s) => s,
        Err(_) => return bad_utf8(),
    };
    let name = match str::from_utf8(name.as_slice()) {
        Ok(s) => s,
        Err(_) => return bad_utf8(),
    };
    let item = item.which.clone();
    handle_result(linker.define(module, name, item), |_linker| ())
}

#[cfg(feature = "wasi")]
#[no_mangle]
pub extern "C" fn wasmtime_linker_define_wasi(
    linker: &mut wasmtime_linker_t,
    instance: &crate::wasi_instance_t,
) -> Option<Box<wasmtime_error_t>> {
    let linker = &mut linker.linker;
    handle_result(instance.add_to_linker(linker), |_linker| ())
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_define_instance(
    linker: &mut wasmtime_linker_t,
    name: &wasm_name_t,
    instance: &wasm_instance_t,
) -> Option<Box<wasmtime_error_t>> {
    let linker = &mut linker.linker;
    let name = match str::from_utf8(name.as_slice()) {
        Ok(s) => s,
        Err(_) => return bad_utf8(),
    };
    handle_result(linker.instance(name, instance.instance()), |_linker| ())
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_instantiate(
    linker: &wasmtime_linker_t,
    module: &wasm_module_t,
    instance_ptr: &mut *mut wasm_instance_t,
    trap_ptr: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    let result = linker.linker.instantiate(module.module());
    super::instance::handle_instantiate(result, instance_ptr, trap_ptr)
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_module(
    linker: &mut wasmtime_linker_t,
    name: &wasm_name_t,
    module: &wasm_module_t,
) -> Option<Box<wasmtime_error_t>> {
    let linker = &mut linker.linker;
    let name = match str::from_utf8(name.as_slice()) {
        Ok(s) => s,
        Err(_) => return bad_utf8(),
    };
    handle_result(linker.module(name, module.module()), |_linker| ())
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_get_default(
    linker: &wasmtime_linker_t,
    name: &wasm_name_t,
    func: &mut *mut wasm_func_t,
) -> Option<Box<wasmtime_error_t>> {
    let linker = &linker.linker;
    let name = match str::from_utf8(name.as_slice()) {
        Ok(s) => s,
        Err(_) => return bad_utf8(),
    };
    handle_result(linker.get_default(name), |f| {
        *func = Box::into_raw(Box::new(f.into()))
    })
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_get_one_by_name(
    linker: &wasmtime_linker_t,
    module: &wasm_name_t,
    name: Option<&wasm_name_t>,
    item_ptr: &mut *mut wasm_extern_t,
) -> Option<Box<wasmtime_error_t>> {
    let linker = &linker.linker;
    let module = match str::from_utf8(module.as_slice()) {
        Ok(s) => s,
        Err(_) => return bad_utf8(),
    };
    let name = match name {
        Some(name) => match str::from_utf8(name.as_slice()) {
            Ok(s) => Some(s),
            Err(_) => return bad_utf8(),
        },
        None => None,
    };
    handle_result(linker.get_one_by_name(module, name), |which| {
        *item_ptr = Box::into_raw(Box::new(wasm_extern_t { which }))
    })
}
