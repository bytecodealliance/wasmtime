use crate::host_ref::HostRef;
use crate::wasm_externkind_t;
use crate::{wasm_externtype_t, wasm_func_t, wasm_global_t, wasm_memory_t, wasm_table_t};
use wasmtime::{ExternType, Func, Global, Memory, Table};

#[derive(Clone)]
pub struct wasm_extern_t {
    pub(crate) which: ExternHost,
}

wasmtime_c_api_macros::declare_ref!(wasm_extern_t);

#[derive(Clone)]
pub(crate) enum ExternHost {
    Func(HostRef<Func>),
    Global(HostRef<Global>),
    Memory(HostRef<Memory>),
    Table(HostRef<Table>),
}

impl wasm_extern_t {
    pub(crate) fn externref(&self) -> wasmtime::ExternRef {
        match &self.which {
            ExternHost::Func(f) => f.clone().into(),
            ExternHost::Global(f) => f.clone().into(),
            ExternHost::Memory(f) => f.clone().into(),
            ExternHost::Table(f) => f.clone().into(),
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_extern_kind(e: &wasm_extern_t) -> wasm_externkind_t {
    match e.which {
        ExternHost::Func(_) => crate::WASM_EXTERN_FUNC,
        ExternHost::Global(_) => crate::WASM_EXTERN_GLOBAL,
        ExternHost::Table(_) => crate::WASM_EXTERN_TABLE,
        ExternHost::Memory(_) => crate::WASM_EXTERN_MEMORY,
    }
}

#[no_mangle]
pub extern "C" fn wasm_extern_type(e: &wasm_extern_t) -> Box<wasm_externtype_t> {
    let ty = match &e.which {
        ExternHost::Func(f) => ExternType::Func(f.borrow().ty()),
        ExternHost::Global(f) => ExternType::Global(f.borrow().ty()),
        ExternHost::Table(f) => ExternType::Table(f.borrow().ty()),
        ExternHost::Memory(f) => ExternType::Memory(f.borrow().ty()),
    };
    Box::new(wasm_externtype_t::new(ty))
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
