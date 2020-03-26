use crate::{wasm_externtype_t, wasm_name_t};
use once_cell::unsync::OnceCell;
use std::str;
use wasmtime::ImportType;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_importtype_t {
    pub(crate) ty: ImportType,
    module_cache: OnceCell<wasm_name_t>,
    name_cache: OnceCell<wasm_name_t>,
    type_cache: OnceCell<wasm_externtype_t>,
}

wasmtime_c_api_macros::declare_ty!(wasm_importtype_t);

impl wasm_importtype_t {
    pub(crate) fn new(ty: ImportType) -> wasm_importtype_t {
        wasm_importtype_t {
            ty,
            module_cache: OnceCell::new(),
            name_cache: OnceCell::new(),
            type_cache: OnceCell::new(),
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_importtype_new(
    module: &mut wasm_name_t,
    name: &mut wasm_name_t,
    ty: Box<wasm_externtype_t>,
) -> Option<Box<wasm_importtype_t>> {
    let module = module.take();
    let name = name.take();
    let module = str::from_utf8(&module).ok()?;
    let name = str::from_utf8(&name).ok()?;
    let ty = ImportType::new(module, name, ty.ty());
    Some(Box::new(wasm_importtype_t::new(ty)))
}

#[no_mangle]
pub extern "C" fn wasm_importtype_module(it: &wasm_importtype_t) -> &wasm_name_t {
    it.module_cache
        .get_or_init(|| wasm_name_t::from_name(&it.ty.module()))
}

#[no_mangle]
pub extern "C" fn wasm_importtype_name(it: &wasm_importtype_t) -> &wasm_name_t {
    it.name_cache
        .get_or_init(|| wasm_name_t::from_name(&it.ty.name()))
}

#[no_mangle]
pub extern "C" fn wasm_importtype_type(it: &wasm_importtype_t) -> &wasm_externtype_t {
    it.type_cache
        .get_or_init(|| wasm_externtype_t::new(it.ty.ty().clone()))
}
