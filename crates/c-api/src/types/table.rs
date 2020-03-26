use crate::{wasm_externtype_t, wasm_limits_t, wasm_valtype_t, CExternType};
use once_cell::unsync::OnceCell;
use wasmtime::TableType;

#[repr(transparent)]
#[derive(Clone)]
pub struct wasm_tabletype_t {
    ext: wasm_externtype_t,
}

wasmtime_c_api_macros::declare_ty!(wasm_tabletype_t);

#[derive(Clone)]
pub(crate) struct CTableType {
    pub(crate) ty: TableType,
    element_cache: OnceCell<wasm_valtype_t>,
    limits_cache: OnceCell<wasm_limits_t>,
}

impl wasm_tabletype_t {
    pub(crate) fn new(ty: TableType) -> wasm_tabletype_t {
        wasm_tabletype_t {
            ext: wasm_externtype_t::new(ty.into()),
        }
    }

    pub(crate) fn try_from(e: &wasm_externtype_t) -> Option<&wasm_tabletype_t> {
        match &e.which {
            CExternType::Table(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    pub(crate) fn ty(&self) -> &CTableType {
        match &self.ext.which {
            CExternType::Table(f) => &f,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

impl CTableType {
    pub(crate) fn new(ty: TableType) -> CTableType {
        CTableType {
            ty,
            element_cache: OnceCell::new(),
            limits_cache: OnceCell::new(),
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_tabletype_new(
    ty: Box<wasm_valtype_t>,
    limits: &wasm_limits_t,
) -> Box<wasm_tabletype_t> {
    Box::new(wasm_tabletype_t::new(TableType::new(
        ty.ty,
        limits.to_wasmtime(),
    )))
}

#[no_mangle]
pub extern "C" fn wasm_tabletype_element(tt: &wasm_tabletype_t) -> &wasm_valtype_t {
    let tt = tt.ty();
    tt.element_cache.get_or_init(|| wasm_valtype_t {
        ty: tt.ty.element().clone(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_tabletype_limits(tt: &wasm_tabletype_t) -> &wasm_limits_t {
    let tt = tt.ty();
    tt.limits_cache.get_or_init(|| {
        let limits = tt.ty.limits();
        wasm_limits_t {
            min: limits.min(),
            max: limits.max().unwrap_or(u32::max_value()),
        }
    })
}

#[no_mangle]
pub extern "C" fn wasm_tabletype_as_externtype(ty: &wasm_tabletype_t) -> &wasm_externtype_t {
    &ty.ext
}

#[no_mangle]
pub extern "C" fn wasm_tabletype_as_externtype_const(ty: &wasm_tabletype_t) -> &wasm_externtype_t {
    &ty.ext
}
