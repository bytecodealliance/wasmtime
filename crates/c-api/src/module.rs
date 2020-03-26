use crate::{wasm_byte_vec_t, wasm_exporttype_vec_t, wasm_importtype_vec_t};
use crate::{wasm_exporttype_t, wasm_importtype_t, wasm_store_t};
use wasmtime::{HostRef, Module};

#[repr(C)]
#[derive(Clone)]
pub struct wasm_module_t {
    pub(crate) module: HostRef<Module>,
    pub(crate) imports: Vec<wasm_importtype_t>,
    pub(crate) exports: Vec<wasm_exporttype_t>,
}

wasmtime_c_api_macros::declare_ref!(wasm_module_t);

impl wasm_module_t {
    fn anyref(&self) -> wasmtime::AnyRef {
        self.module.anyref()
    }
}

#[no_mangle]
pub extern "C" fn wasm_module_new(
    store: &wasm_store_t,
    binary: &wasm_byte_vec_t,
) -> Option<Box<wasm_module_t>> {
    let binary = binary.as_slice();
    let store = &store.store.borrow();
    let module = Module::from_binary(store, binary).ok()?;
    let imports = module
        .imports()
        .iter()
        .map(|i| wasm_importtype_t::new(i.clone()))
        .collect::<Vec<_>>();
    let exports = module
        .exports()
        .iter()
        .map(|e| wasm_exporttype_t::new(e.clone()))
        .collect::<Vec<_>>();
    Some(Box::new(wasm_module_t {
        module: HostRef::new(module),
        imports,
        exports,
    }))
}

#[no_mangle]
pub extern "C" fn wasm_module_validate(store: &wasm_store_t, binary: &wasm_byte_vec_t) -> bool {
    let binary = binary.as_slice();
    let store = &store.store.borrow();
    Module::validate(store, binary).is_ok()
}

#[no_mangle]
pub extern "C" fn wasm_module_exports(module: &wasm_module_t, out: &mut wasm_exporttype_vec_t) {
    let buffer = module
        .exports
        .iter()
        .map(|et| Some(Box::new(et.clone())))
        .collect::<Vec<_>>();
    out.set_buffer(buffer);
}

#[no_mangle]
pub extern "C" fn wasm_module_imports(module: &wasm_module_t, out: &mut wasm_importtype_vec_t) {
    let buffer = module
        .imports
        .iter()
        .map(|it| Some(Box::new(it.clone())))
        .collect::<Vec<_>>();
    out.set_buffer(buffer);
}
