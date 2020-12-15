use crate::{wasm_exporttype_t, wasm_exporttype_vec_t, wasm_externtype_t, CExternType};
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
}

impl wasm_instancetype_t {
    pub(crate) fn new(ty: InstanceType) -> wasm_instancetype_t {
        wasm_instancetype_t {
            ext: wasm_externtype_t::new(ty.into()),
        }
    }

    pub(crate) fn try_from(e: &wasm_externtype_t) -> Option<&wasm_instancetype_t> {
        match &e.which {
            CExternType::Instance(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    pub(crate) fn ty(&self) -> &CInstanceType {
        match &self.ext.which {
            CExternType::Instance(f) => &f,
            _ => unreachable!(),
        }
    }
}

impl CInstanceType {
    pub(crate) fn new(ty: InstanceType) -> CInstanceType {
        CInstanceType { ty }
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

#[no_mangle]
pub extern "C" fn wasm_instancetype_exports(
    instance: &wasm_instancetype_t,
    out: &mut wasm_exporttype_vec_t,
) {
    let exports = instance
        .ty()
        .ty
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
