use crate::{wasm_externtype_t, wasm_name_t};
use once_cell::unsync::OnceCell;
use wasmtime::ExternType;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_exporttype_t {
    name: String,
    ty: ExternType,
    name_cache: OnceCell<wasm_name_t>,
    type_cache: OnceCell<wasm_externtype_t>,
}

wasmtime_c_api_macros::declare_ty!(wasm_exporttype_t);

impl wasm_exporttype_t {
    pub(crate) fn new(name: String, ty: ExternType) -> wasm_exporttype_t {
        wasm_exporttype_t {
            name,
            ty,
            name_cache: OnceCell::new(),
            type_cache: OnceCell::new(),
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_exporttype_new(
    name: &mut wasm_name_t,
    ty: Box<wasm_externtype_t>,
) -> Option<Box<wasm_exporttype_t>> {
    let name = name.take();
    let name = String::from_utf8(name).ok()?;
    Some(Box::new(wasm_exporttype_t::new(name, ty.ty())))
}

#[no_mangle]
pub extern "C" fn wasm_exporttype_name(et: &wasm_exporttype_t) -> &wasm_name_t {
    et.name_cache
        .get_or_init(|| wasm_name_t::from_name(et.name.clone()))
}

#[no_mangle]
pub extern "C" fn wasm_exporttype_type(et: &wasm_exporttype_t) -> &wasm_externtype_t {
    et.type_cache
        .get_or_init(|| wasm_externtype_t::new(et.ty.clone()))
}
