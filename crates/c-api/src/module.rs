use crate::{
    handle_result, wasm_byte_vec_t, wasm_engine_t, wasm_exporttype_t, wasm_exporttype_vec_t,
    wasm_extern_t, wasm_importtype_t, wasm_importtype_vec_t, wasm_store_t, wasmtime_error_t,
    wasmtime_moduletype_t, StoreRef,
};
use wasmtime::{Engine, Extern, Module};

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_module_t {
    ext: wasm_extern_t,
}

wasmtime_c_api_macros::declare_ref!(wasm_module_t);

impl wasm_module_t {
    pub(crate) fn new(store: StoreRef, module: Module) -> wasm_module_t {
        wasm_module_t {
            ext: wasm_extern_t {
                store: store,
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
pub unsafe extern "C" fn wasm_module_new(
    store: &mut wasm_store_t,
    binary: &wasm_byte_vec_t,
) -> Option<Box<wasm_module_t>> {
    match Module::from_binary(store.store.context().engine(), binary.as_slice()) {
        Ok(module) => Some(Box::new(wasm_module_t::new(store.store.clone(), module))),
        Err(_) => None,
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_module_validate(
    store: &mut wasm_store_t,
    binary: &wasm_byte_vec_t,
) -> bool {
    Module::validate(store.store.context().engine(), binary.as_slice()).is_ok()
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
pub unsafe extern "C" fn wasm_module_obtain(
    store: &mut wasm_store_t,
    shared_module: &wasm_shared_module_t,
) -> Option<Box<wasm_module_t>> {
    let module = shared_module.module.clone();
    if Engine::same(store.store.context().engine(), module.engine()) {
        Some(Box::new(wasm_module_t::new(store.store.clone(), module)))
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn wasm_module_serialize(module: &wasm_module_t, ret: &mut wasm_byte_vec_t) {
    if let Ok(buf) = module.module().serialize() {
        ret.set_buffer(buf);
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_module_deserialize(
    store: &mut wasm_store_t,
    binary: &wasm_byte_vec_t,
) -> Option<Box<wasm_module_t>> {
    match Module::deserialize(store.store.context().engine(), binary.as_slice()) {
        Ok(module) => Some(Box::new(wasm_module_t::new(store.store.clone(), module))),
        Err(_) => None,
    }
}

#[derive(Clone)]
pub struct wasmtime_module_t {
    pub(crate) module: Module,
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_module_new(
    engine: &wasm_engine_t,
    wasm: *const u8,
    len: usize,
    out: &mut *mut wasmtime_module_t,
) -> Option<Box<wasmtime_error_t>> {
    handle_result(
        Module::from_binary(&engine.engine, crate::slice_from_raw_parts(wasm, len)),
        |module| {
            *out = Box::into_raw(Box::new(wasmtime_module_t { module }));
        },
    )
}

#[no_mangle]
pub extern "C" fn wasmtime_module_delete(_module: Box<wasmtime_module_t>) {}

#[no_mangle]
pub extern "C" fn wasmtime_module_clone(module: &wasmtime_module_t) -> Box<wasmtime_module_t> {
    Box::new(module.clone())
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_module_validate(
    engine: &wasm_engine_t,
    wasm: *const u8,
    len: usize,
) -> Option<Box<wasmtime_error_t>> {
    let binary = crate::slice_from_raw_parts(wasm, len);
    handle_result(Module::validate(&engine.engine, binary), |()| {})
}

#[no_mangle]
pub extern "C" fn wasmtime_module_type(m: &wasmtime_module_t) -> Box<wasmtime_moduletype_t> {
    Box::new(wasmtime_moduletype_t::new(m.module.ty()))
}

#[no_mangle]
pub extern "C" fn wasmtime_module_serialize(
    module: &wasmtime_module_t,
    ret: &mut wasm_byte_vec_t,
) -> Option<Box<wasmtime_error_t>> {
    handle_result(module.module.serialize(), |buf| ret.set_buffer(buf))
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_module_deserialize(
    engine: &wasm_engine_t,
    bytes: *const u8,
    len: usize,
    out: &mut *mut wasmtime_module_t,
) -> Option<Box<wasmtime_error_t>> {
    let bytes = crate::slice_from_raw_parts(bytes, len);
    handle_result(Module::deserialize(&engine.engine, bytes), |module| {
        *out = Box::into_raw(Box::new(wasmtime_module_t { module }));
    })
}
