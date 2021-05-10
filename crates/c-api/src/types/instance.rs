use crate::{wasm_exporttype_t, wasm_exporttype_vec_t, wasm_externtype_t, CExternType};
use wasmtime::InstanceType;

#[repr(transparent)]
#[derive(Clone)]
pub struct wasmtime_instancetype_t {
    ext: wasm_externtype_t,
}

wasmtime_c_api_macros::declare_ty!(wasmtime_instancetype_t);

#[derive(Clone)]
pub(crate) struct CInstanceType {
    pub(crate) ty: InstanceType,
}

impl wasmtime_instancetype_t {
    pub(crate) fn new(ty: InstanceType) -> wasmtime_instancetype_t {
        wasmtime_instancetype_t {
            ext: wasm_externtype_t::new(ty.into()),
        }
    }

    pub(crate) fn try_from(e: &wasm_externtype_t) -> Option<&wasmtime_instancetype_t> {
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
pub extern "C" fn wasmtime_instancetype_as_externtype(
    ty: &wasmtime_instancetype_t,
) -> &wasm_externtype_t {
    &ty.ext
}

#[no_mangle]
pub extern "C" fn wasmtime_instancetype_exports(
    instance: &wasmtime_instancetype_t,
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
