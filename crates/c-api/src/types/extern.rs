use crate::{wasm_functype_t, wasm_globaltype_t, wasm_memorytype_t, wasm_tabletype_t};
use crate::{wasmtime_instancetype_t, wasmtime_moduletype_t};
use crate::{CFuncType, CGlobalType, CInstanceType, CMemoryType, CModuleType, CTableType};
use wasmtime::ExternType;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_externtype_t {
    pub(crate) which: CExternType,
}

wasmtime_c_api_macros::declare_ty!(wasm_externtype_t);

#[derive(Clone)]
pub(crate) enum CExternType {
    Func(CFuncType),
    Global(CGlobalType),
    Memory(CMemoryType),
    Table(CTableType),
    Instance(CInstanceType),
    Module(CModuleType),
}

pub type wasm_externkind_t = u8;

pub const WASM_EXTERN_FUNC: wasm_externkind_t = 0;
pub const WASM_EXTERN_GLOBAL: wasm_externkind_t = 1;
pub const WASM_EXTERN_TABLE: wasm_externkind_t = 2;
pub const WASM_EXTERN_MEMORY: wasm_externkind_t = 3;
pub const WASM_EXTERN_MODULE: wasm_externkind_t = 4;
pub const WASM_EXTERN_INSTANCE: wasm_externkind_t = 5;

impl wasm_externtype_t {
    pub(crate) fn new(ty: ExternType) -> wasm_externtype_t {
        wasm_externtype_t {
            which: match ty {
                ExternType::Func(f) => CExternType::Func(CFuncType::new(f)),
                ExternType::Global(f) => CExternType::Global(CGlobalType::new(f)),
                ExternType::Memory(f) => CExternType::Memory(CMemoryType::new(f)),
                ExternType::Table(f) => CExternType::Table(CTableType::new(f)),
                ExternType::Instance(f) => CExternType::Instance(CInstanceType::new(f)),
                ExternType::Module(f) => CExternType::Module(CModuleType::new(f)),
            },
        }
    }

    pub(crate) fn ty(&self) -> ExternType {
        match &self.which {
            CExternType::Func(f) => ExternType::Func(f.ty.clone()),
            CExternType::Table(f) => ExternType::Table(f.ty.clone()),
            CExternType::Global(f) => ExternType::Global(f.ty.clone()),
            CExternType::Memory(f) => ExternType::Memory(f.ty.clone()),
            CExternType::Instance(f) => ExternType::Instance(f.ty.clone()),
            CExternType::Module(f) => ExternType::Module(f.ty.clone()),
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_externtype_kind(et: &wasm_externtype_t) -> wasm_externkind_t {
    match &et.which {
        CExternType::Func(_) => WASM_EXTERN_FUNC,
        CExternType::Table(_) => WASM_EXTERN_TABLE,
        CExternType::Global(_) => WASM_EXTERN_GLOBAL,
        CExternType::Memory(_) => WASM_EXTERN_MEMORY,
        CExternType::Instance(_) => WASM_EXTERN_INSTANCE,
        CExternType::Module(_) => WASM_EXTERN_MODULE,
    }
}

#[no_mangle]
pub extern "C" fn wasm_externtype_as_functype(et: &wasm_externtype_t) -> Option<&wasm_functype_t> {
    wasm_externtype_as_functype_const(et)
}

#[no_mangle]
pub extern "C" fn wasm_externtype_as_functype_const(
    et: &wasm_externtype_t,
) -> Option<&wasm_functype_t> {
    wasm_functype_t::try_from(et)
}

#[no_mangle]
pub extern "C" fn wasm_externtype_as_globaltype(
    et: &wasm_externtype_t,
) -> Option<&wasm_globaltype_t> {
    wasm_externtype_as_globaltype_const(et)
}

#[no_mangle]
pub extern "C" fn wasm_externtype_as_globaltype_const(
    et: &wasm_externtype_t,
) -> Option<&wasm_globaltype_t> {
    wasm_globaltype_t::try_from(et)
}

#[no_mangle]
pub extern "C" fn wasm_externtype_as_tabletype(
    et: &wasm_externtype_t,
) -> Option<&wasm_tabletype_t> {
    wasm_externtype_as_tabletype_const(et)
}

#[no_mangle]
pub extern "C" fn wasm_externtype_as_tabletype_const(
    et: &wasm_externtype_t,
) -> Option<&wasm_tabletype_t> {
    wasm_tabletype_t::try_from(et)
}

#[no_mangle]
pub extern "C" fn wasm_externtype_as_memorytype(
    et: &wasm_externtype_t,
) -> Option<&wasm_memorytype_t> {
    wasm_externtype_as_memorytype_const(et)
}

#[no_mangle]
pub extern "C" fn wasm_externtype_as_memorytype_const(
    et: &wasm_externtype_t,
) -> Option<&wasm_memorytype_t> {
    wasm_memorytype_t::try_from(et)
}

#[no_mangle]
pub extern "C" fn wasmtime_externtype_as_moduletype(
    et: &wasm_externtype_t,
) -> Option<&wasmtime_moduletype_t> {
    wasmtime_moduletype_t::try_from(et)
}

#[no_mangle]
pub extern "C" fn wasmtime_externtype_as_instancetype(
    et: &wasm_externtype_t,
) -> Option<&wasmtime_instancetype_t> {
    wasmtime_instancetype_t::try_from(et)
}
