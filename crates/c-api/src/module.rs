use crate::{
    handle_result, wasm_byte_vec_t, wasm_engine_t, wasm_exporttype_t, wasm_exporttype_vec_t,
    wasm_importtype_t, wasm_importtype_vec_t, wasm_store_t, wasmtime_error_t,
};
use std::ptr;
use wasmtime::{Engine, Module};

#[repr(C)]
#[derive(Clone)]
pub struct wasm_module_t {
    pub(crate) module: Module,
    pub(crate) imports: Vec<wasm_importtype_t>,
    pub(crate) exports: Vec<wasm_exporttype_t>,
}

wasmtime_c_api_macros::declare_ref!(wasm_module_t);

#[repr(C)]
#[derive(Clone)]
pub struct wasm_shared_module_t {
    module: Module,
}

wasmtime_c_api_macros::declare_own!(wasm_shared_module_t);

#[no_mangle]
pub extern "C" fn wasm_module_new(
    store: &wasm_store_t,
    binary: &wasm_byte_vec_t,
) -> Option<Box<wasm_module_t>> {
    let mut ret = ptr::null_mut();
    let engine = wasm_engine_t {
        engine: store.store.engine().clone(),
    };
    match wasmtime_module_new(&engine, binary, &mut ret) {
        Some(_err) => None,
        None => {
            assert!(!ret.is_null());
            Some(unsafe { Box::from_raw(ret) })
        }
    }
}

#[no_mangle]
pub extern "C" fn wasmtime_module_new(
    engine: &wasm_engine_t,
    binary: &wasm_byte_vec_t,
    ret: &mut *mut wasm_module_t,
) -> Option<Box<wasmtime_error_t>> {
    let binary = binary.as_slice();
    handle_result(Module::from_binary(&engine.engine, binary), |module| {
        let imports = module
            .imports()
            .map(|i| wasm_importtype_t::new(i.module().to_owned(), i.name().to_owned(), i.ty()))
            .collect::<Vec<_>>();
        let exports = module
            .exports()
            .map(|e| wasm_exporttype_t::new(e.name().to_owned(), e.ty()))
            .collect::<Vec<_>>();
        let module = Box::new(wasm_module_t {
            module: module,
            imports,
            exports,
        });
        *ret = Box::into_raw(module);
    })
}

#[no_mangle]
pub extern "C" fn wasm_module_validate(store: &wasm_store_t, binary: &wasm_byte_vec_t) -> bool {
    wasmtime_module_validate(store, binary).is_none()
}

#[no_mangle]
pub extern "C" fn wasmtime_module_validate(
    store: &wasm_store_t,
    binary: &wasm_byte_vec_t,
) -> Option<Box<wasmtime_error_t>> {
    let binary = binary.as_slice();
    handle_result(Module::validate(store.store.engine(), binary), |()| {})
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

#[no_mangle]
pub extern "C" fn wasm_module_share(module: &wasm_module_t) -> Box<wasm_shared_module_t> {
    Box::new(wasm_shared_module_t {
        module: module.module.clone(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_module_obtain(
    store: &wasm_store_t,
    shared_module: &wasm_shared_module_t,
) -> Option<Box<wasm_module_t>> {
    let module = shared_module.module.clone();
    if !Engine::same(store.store.engine(), module.engine()) {
        return None;
    }
    let imports = module
        .imports()
        .map(|i| wasm_importtype_t::new(i.module().to_owned(), i.name().to_owned(), i.ty()))
        .collect::<Vec<_>>();
    let exports = module
        .exports()
        .map(|e| wasm_exporttype_t::new(e.name().to_owned(), e.ty()))
        .collect::<Vec<_>>();
    Some(Box::new(wasm_module_t {
        module: module,
        imports,
        exports,
    }))
}

#[no_mangle]
pub extern "C" fn wasm_module_serialize(module: &wasm_module_t, ret: &mut wasm_byte_vec_t) {
    drop(wasmtime_module_serialize(module, ret));
}

#[no_mangle]
pub extern "C" fn wasm_module_deserialize(
    store: &wasm_store_t,
    binary: &wasm_byte_vec_t,
) -> Option<Box<wasm_module_t>> {
    let mut ret = ptr::null_mut();
    let engine = wasm_engine_t {
        engine: store.store.engine().clone(),
    };
    match wasmtime_module_deserialize(&engine, binary, &mut ret) {
        Some(_err) => None,
        None => {
            assert!(!ret.is_null());
            Some(unsafe { Box::from_raw(ret) })
        }
    }
}

#[no_mangle]
pub extern "C" fn wasmtime_module_serialize(
    module: &wasm_module_t,
    ret: &mut wasm_byte_vec_t,
) -> Option<Box<wasmtime_error_t>> {
    handle_result(module.module.serialize(), |buf| {
        ret.set_buffer(buf);
    })
}

#[no_mangle]
pub extern "C" fn wasmtime_module_deserialize(
    engine: &wasm_engine_t,
    binary: &wasm_byte_vec_t,
    ret: &mut *mut wasm_module_t,
) -> Option<Box<wasmtime_error_t>> {
    handle_result(
        Module::deserialize(&engine.engine, binary.as_slice()),
        |module| {
            let imports = module
                .imports()
                .map(|i| wasm_importtype_t::new(i.module().to_owned(), i.name().to_owned(), i.ty()))
                .collect::<Vec<_>>();
            let exports = module
                .exports()
                .map(|e| wasm_exporttype_t::new(e.name().to_owned(), e.ty()))
                .collect::<Vec<_>>();
            let module = Box::new(wasm_module_t {
                module: module,
                imports,
                exports,
            });
            *ret = Box::into_raw(module);
        },
    )
}
