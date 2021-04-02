use crate::{
    handle_result, wasm_byte_vec_t, wasm_engine_t, wasm_exporttype_t, wasm_exporttype_vec_t,
    wasm_extern_t, wasm_importtype_t, wasm_importtype_vec_t, wasm_moduletype_t, wasm_store_t,
    wasmtime_error_t,
};
use std::ptr;
use wasmtime::{Engine, Extern, Module};

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_module_t {
    ext: wasm_extern_t,
}

wasmtime_c_api_macros::declare_ref!(wasm_module_t);

impl wasm_module_t {
    pub(crate) fn new(module: Module) -> wasm_module_t {
        wasm_module_t {
            ext: wasm_extern_t {
                which: module.into(),
            },
        }
    }

    pub(crate) fn try_from(e: &wasm_extern_t) -> Option<&wasm_module_t> {
        match &e.which {
            Extern::Module(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    pub(crate) fn module(&self) -> &Module {
        match &self.ext.which {
            Extern::Module(i) => i,
            _ => unreachable!(),
        }
    }
}

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
        let module = Box::new(wasm_module_t::new(module));
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
pub extern "C" fn wasm_module_as_extern(m: &wasm_module_t) -> &wasm_extern_t {
    &m.ext
}

#[no_mangle]
pub extern "C" fn wasm_module_exports(module: &wasm_module_t, out: &mut wasm_exporttype_vec_t) {
    let exports = module
        .module()
        .exports()
        .map(|e| {
            Some(Box::new(wasm_exporttype_t::new(
                e.name().to_owned(),
                e.ty(),
            )))
        })
        .collect::<Vec<_>>();
    out.set_buffer(exports);
}

#[no_mangle]
pub extern "C" fn wasm_module_imports(module: &wasm_module_t, out: &mut wasm_importtype_vec_t) {
    let imports = module
        .module()
        .imports()
        .map(|i| {
            Some(Box::new(wasm_importtype_t::new(
                i.module().to_owned(),
                i.name().map(|s| s.to_owned()),
                i.ty(),
            )))
        })
        .collect::<Vec<_>>();
    out.set_buffer(imports);
}

#[no_mangle]
pub extern "C" fn wasm_module_share(module: &wasm_module_t) -> Box<wasm_shared_module_t> {
    Box::new(wasm_shared_module_t {
        module: module.module().clone(),
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
    Some(Box::new(wasm_module_t::new(module)))
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
    handle_result(module.module().serialize(), |buf| {
        ret.set_buffer(buf);
    })
}

#[no_mangle]
pub extern "C" fn wasmtime_module_deserialize(
    engine: &wasm_engine_t,
    binary: &wasm_byte_vec_t,
    ret: &mut *mut wasm_module_t,
) -> Option<Box<wasmtime_error_t>> {
    handle_result(Module::new(&engine.engine, binary.as_slice()), |module| {
        let module = Box::new(wasm_module_t::new(module));
        *ret = Box::into_raw(module);
    })
}

#[no_mangle]
pub extern "C" fn wasm_module_type(f: &wasm_module_t) -> Box<wasm_moduletype_t> {
    Box::new(wasm_moduletype_t::new(f.module().ty()))
}
