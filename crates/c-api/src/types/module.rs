use crate::{
    wasm_exporttype_t, wasm_exporttype_vec_t, wasm_externtype_t, wasm_importtype_t,
    wasm_importtype_vec_t, CExternType,
};
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
}

impl wasm_moduletype_t {
    pub(crate) fn new(ty: ModuleType) -> wasm_moduletype_t {
        wasm_moduletype_t {
            ext: wasm_externtype_t::new(ty.into()),
        }
    }

    pub(crate) fn try_from(e: &wasm_externtype_t) -> Option<&wasm_moduletype_t> {
        match &e.which {
            CExternType::Module(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    pub(crate) fn ty(&self) -> &CModuleType {
        match &self.ext.which {
            CExternType::Module(f) => &f,
            _ => unreachable!(),
        }
    }
}

impl CModuleType {
    pub(crate) fn new(ty: ModuleType) -> CModuleType {
        CModuleType { ty }
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

#[no_mangle]
pub extern "C" fn wasm_moduletype_exports(
    module: &wasm_moduletype_t,
    out: &mut wasm_exporttype_vec_t,
) {
    let exports = module
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

#[no_mangle]
pub extern "C" fn wasm_moduletype_imports(
    module: &wasm_moduletype_t,
    out: &mut wasm_importtype_vec_t,
) {
    let imports = module
        .ty()
        .ty
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
