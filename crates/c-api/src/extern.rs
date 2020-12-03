use crate::wasm_externkind_t;
use crate::{wasm_externtype_t, wasm_func_t, wasm_global_t, wasm_memory_t, wasm_table_t};
use wasmtime::Extern;

#[derive(Clone)]
pub struct wasm_extern_t {
    pub(crate) which: Extern,
}

wasmtime_c_api_macros::declare_ref!(wasm_extern_t);

#[no_mangle]
pub extern "C" fn wasm_extern_kind(e: &wasm_extern_t) -> wasm_externkind_t {
    match e.which {
        Extern::Func(_) => crate::WASM_EXTERN_FUNC,
        Extern::Global(_) => crate::WASM_EXTERN_GLOBAL,
        Extern::Table(_) => crate::WASM_EXTERN_TABLE,
        Extern::Memory(_) => crate::WASM_EXTERN_MEMORY,

        // FIXME(#2094)
        Extern::Instance(_) => unimplemented!(),
        Extern::Module(_) => unimplemented!(),
    }
}

#[no_mangle]
pub extern "C" fn wasm_extern_type(e: &wasm_extern_t) -> Box<wasm_externtype_t> {
    Box::new(wasm_externtype_t::new(e.which.ty()))
}

#[no_mangle]
pub extern "C" fn wasm_extern_as_func(e: &wasm_extern_t) -> Option<&wasm_func_t> {
    wasm_func_t::try_from(e)
}

#[no_mangle]
pub extern "C" fn wasm_extern_as_func_const(e: &wasm_extern_t) -> Option<&wasm_func_t> {
    wasm_extern_as_func(e)
}

#[no_mangle]
pub extern "C" fn wasm_extern_as_global(e: &wasm_extern_t) -> Option<&wasm_global_t> {
    wasm_global_t::try_from(e)
}

#[no_mangle]
pub extern "C" fn wasm_extern_as_global_const(e: &wasm_extern_t) -> Option<&wasm_global_t> {
    wasm_extern_as_global(e)
}

#[no_mangle]
pub extern "C" fn wasm_extern_as_table(e: &wasm_extern_t) -> Option<&wasm_table_t> {
    wasm_table_t::try_from(e)
}

#[no_mangle]
pub extern "C" fn wasm_extern_as_table_const(e: &wasm_extern_t) -> Option<&wasm_table_t> {
    wasm_extern_as_table(e)
}

#[no_mangle]
pub extern "C" fn wasm_extern_as_memory(e: &wasm_extern_t) -> Option<&wasm_memory_t> {
    wasm_memory_t::try_from(e)
}

#[no_mangle]
pub extern "C" fn wasm_extern_as_memory_const(e: &wasm_extern_t) -> Option<&wasm_memory_t> {
    wasm_extern_as_memory(e)
}
