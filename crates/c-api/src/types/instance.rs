use crate::{wasm_externtype_t, wasm_limits_t, CExternType};
use once_cell::unsync::OnceCell;
use wasmtime::InstanceType;

#[repr(transparent)]
#[derive(Clone)]
pub struct wasm_instancetype_t {
    ext: wasm_externtype_t,
}

wasmtime_c_api_macros::declare_ty!(wasm_instancetype_t);

#[derive(Clone)]
pub(crate) struct CInstanceType {
    pub(crate) ty: InstanceType,
    limits_cache: OnceCell<wasm_limits_t>,
}

impl wasm_instancetype_t {
    pub(crate) fn try_from(e: &wasm_externtype_t) -> Option<&wasm_instancetype_t> {
        match &e.which {
            CExternType::Instance(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }
}

impl CInstanceType {
    pub(crate) fn new(ty: InstanceType) -> CInstanceType {
        CInstanceType {
            ty,
            limits_cache: OnceCell::new(),
        }
    }
}
#[no_mangle]
pub extern "C" fn wasm_instancetype_as_externtype(ty: &wasm_instancetype_t) -> &wasm_externtype_t {
    &ty.ext
}

#[no_mangle]
pub extern "C" fn wasm_instancetype_as_externtype_const(
    ty: &wasm_instancetype_t,
) -> &wasm_externtype_t {
    &ty.ext
}
