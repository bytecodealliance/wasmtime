use crate::{wasm_limits_t, wasm_valtype_t};
use once_cell::unsync::OnceCell;
use wasmtime::TableType;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_tabletype_t {
    pub(crate) tabletype: TableType,
    element_cache: OnceCell<wasm_valtype_t>,
    limits_cache: OnceCell<wasm_limits_t>,
}

impl wasm_tabletype_t {
    pub(crate) fn new(tabletype: TableType) -> wasm_tabletype_t {
        wasm_tabletype_t {
            tabletype,
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
    tt.element_cache.get_or_init(|| wasm_valtype_t {
        ty: tt.tabletype.element().clone(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_tabletype_limits(tt: &wasm_tabletype_t) -> &wasm_limits_t {
    tt.limits_cache.get_or_init(|| {
        let limits = tt.tabletype.limits();
        wasm_limits_t {
            min: limits.min(),
            max: limits.max().unwrap_or(u32::max_value()),
        }
    })
}

#[no_mangle]
pub extern "C" fn wasm_tabletype_delete(_tt: Box<wasm_tabletype_t>) {}
