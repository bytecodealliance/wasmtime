use crate::{wasi_instance_t, wasm_extern_t, wasm_store_t, ExternHost};
use crate::{wasm_instance_t, wasm_module_t, wasm_name_t, wasm_trap_t};
use std::str;
use wasmtime::{Extern, Linker};

#[repr(C)]
pub struct wasmtime_linker_t {
    linker: Linker,
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_new(store: &wasm_store_t) -> Box<wasmtime_linker_t> {
    Box::new(wasmtime_linker_t {
        linker: Linker::new(&store.store.borrow()),
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
) -> bool {
    let linker = &mut linker.linker;
    let module = match str::from_utf8(module.as_slice()) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let name = match str::from_utf8(name.as_slice()) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let item = match &item.which {
        ExternHost::Func(e) => Extern::Func(e.borrow().clone()),
        ExternHost::Table(e) => Extern::Table(e.borrow().clone()),
        ExternHost::Global(e) => Extern::Global(e.borrow().clone()),
        ExternHost::Memory(e) => Extern::Memory(e.borrow().clone()),
    };
    linker.define(module, name, item).is_ok()
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_define_wasi(
    linker: &mut wasmtime_linker_t,
    instance: &wasi_instance_t,
) -> bool {
    let linker = &mut linker.linker;
    instance.add_to_linker(linker).is_ok()
}

#[no_mangle]
pub extern "C" fn wasmtime_linker_define_instance(
    linker: &mut wasmtime_linker_t,
    name: &wasm_name_t,
    instance: &wasm_instance_t,
) -> bool {
    let linker = &mut linker.linker;
    let name = match str::from_utf8(name.as_slice()) {
        Ok(s) => s,
        Err(_) => return false,
    };
    linker.instance(name, &instance.instance.borrow()).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_linker_instantiate(
    linker: &wasmtime_linker_t,
    module: &wasm_module_t,
    trap: Option<&mut *mut wasm_trap_t>,
) -> Option<Box<wasm_instance_t>> {
    let linker = &linker.linker;
    let result = linker.instantiate(&module.module.borrow());
    super::instance::handle_instantiate(result, trap)
}
