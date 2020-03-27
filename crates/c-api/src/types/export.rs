use crate::{wasm_externtype_t, wasm_name_t};
use once_cell::unsync::OnceCell;
use std::str;
use wasmtime::ExportType;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_exporttype_t {
    ty: ExportType,
    name_cache: OnceCell<wasm_name_t>,
    type_cache: OnceCell<wasm_externtype_t>,
}

wasmtime_c_api_macros::declare_ty!(wasm_exporttype_t);

impl wasm_exporttype_t {
    pub(crate) fn new(ty: ExportType) -> wasm_exporttype_t {
        wasm_exporttype_t {
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
    let name = str::from_utf8(&name).ok()?;
    let ty = ExportType::new(name, ty.ty());
    Some(Box::new(wasm_exporttype_t::new(ty)))
}

#[no_mangle]
pub extern "C" fn wasm_exporttype_name(et: &wasm_exporttype_t) -> &wasm_name_t {
    et.name_cache
        .get_or_init(|| wasm_name_t::from_name(&et.ty.name()))
}

#[no_mangle]
pub extern "C" fn wasm_exporttype_type(et: &wasm_exporttype_t) -> &wasm_externtype_t {
    et.type_cache
        .get_or_init(|| wasm_externtype_t::new(et.ty.ty().clone()))
}
