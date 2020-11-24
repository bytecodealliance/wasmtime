use crate::{wasm_externtype_t, wasm_name_t};
use once_cell::unsync::OnceCell;
use wasmtime::ExternType;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_importtype_t {
    pub(crate) module: String,
    pub(crate) name: Option<String>,
    pub(crate) ty: ExternType,
    module_cache: OnceCell<wasm_name_t>,
    name_cache: OnceCell<wasm_name_t>,
    type_cache: OnceCell<wasm_externtype_t>,
}

wasmtime_c_api_macros::declare_ty!(wasm_importtype_t);

impl wasm_importtype_t {
    pub(crate) fn new(module: String, name: Option<String>, ty: ExternType) -> wasm_importtype_t {
        wasm_importtype_t {
            module,
            name,
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
    name: Option<&mut wasm_name_t>,
    ty: Box<wasm_externtype_t>,
) -> Option<Box<wasm_importtype_t>> {
    let module = module.take();
    let name = name.map(|n| n.take());
    let module = String::from_utf8(module).ok()?;
    let name = match name {
        Some(name) => Some(String::from_utf8(name).ok()?),
        None => None,
    };
    Some(Box::new(wasm_importtype_t::new(module, name, ty.ty())))
}

#[no_mangle]
pub extern "C" fn wasm_importtype_module(it: &wasm_importtype_t) -> &wasm_name_t {
    it.module_cache
        .get_or_init(|| wasm_name_t::from_name(it.module.clone()))
}

#[no_mangle]
pub extern "C" fn wasm_importtype_name(it: &wasm_importtype_t) -> Option<&wasm_name_t> {
    let name = it.name.as_ref()?;
    Some(
        it.name_cache
            .get_or_init(|| wasm_name_t::from_name(name.to_string())),
    )
}

#[no_mangle]
pub extern "C" fn wasm_importtype_type(it: &wasm_importtype_t) -> &wasm_externtype_t {
    it.type_cache
        .get_or_init(|| wasm_externtype_t::new(it.ty.clone()))
}
