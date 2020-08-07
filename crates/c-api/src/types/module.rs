use crate::{wasm_externtype_t, wasm_limits_t, CExternType};
use once_cell::unsync::OnceCell;
use wasmtime::ModuleType;

#[repr(transparent)]
#[derive(Clone)]
pub struct wasm_moduletype_t {
    ext: wasm_externtype_t,
}

wasmtime_c_api_macros::declare_ty!(wasm_moduletype_t);

#[derive(Clone)]
pub(crate) struct CModuleType {
    pub(crate) ty: ModuleType,
    limits_cache: OnceCell<wasm_limits_t>,
}

impl wasm_moduletype_t {
    pub(crate) fn try_from(e: &wasm_externtype_t) -> Option<&wasm_moduletype_t> {
        match &e.which {
            CExternType::Module(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }
}

impl CModuleType {
    pub(crate) fn new(ty: ModuleType) -> CModuleType {
        CModuleType {
            ty,
            limits_cache: OnceCell::new(),
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_moduletype_as_externtype(ty: &wasm_moduletype_t) -> &wasm_externtype_t {
    &ty.ext
}

#[no_mangle]
pub extern "C" fn wasm_moduletype_as_externtype_const(
    ty: &wasm_moduletype_t,
) -> &wasm_externtype_t {
    &ty.ext
}
