use crate::{wasm_functype_t, wasm_globaltype_t, wasm_memorytype_t, wasm_tabletype_t};
use once_cell::unsync::OnceCell;
use wasmtime::ExternType;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_externtype_t {
    pub(crate) ty: ExternType,
    cache: OnceCell<Cache>,
}

#[derive(Clone)]
enum Cache {
    Func(wasm_functype_t),
    Global(wasm_globaltype_t),
    Memory(wasm_memorytype_t),
    Table(wasm_tabletype_t),
}

pub type wasm_externkind_t = u8;

pub const WASM_EXTERN_FUNC: wasm_externkind_t = 0;
pub const WASM_EXTERN_GLOBAL: wasm_externkind_t = 1;
pub const WASM_EXTERN_TABLE: wasm_externkind_t = 2;
pub const WASM_EXTERN_MEMORY: wasm_externkind_t = 3;

impl wasm_externtype_t {
    pub(crate) fn new(ty: ExternType) -> wasm_externtype_t {
        wasm_externtype_t {
            ty,
            cache: OnceCell::new(),
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_externtype_kind(et: &wasm_externtype_t) -> wasm_externkind_t {
    match &et.ty {
        ExternType::Func(_) => WASM_EXTERN_FUNC,
        ExternType::Table(_) => WASM_EXTERN_TABLE,
        ExternType::Global(_) => WASM_EXTERN_GLOBAL,
        ExternType::Memory(_) => WASM_EXTERN_MEMORY,
    }
}

#[no_mangle]
pub extern "C" fn wasm_externtype_as_functype(
    et: &wasm_externtype_t,
) -> Option<&wasm_functype_t> {
    wasm_externtype_as_functype_const(et)
}

#[no_mangle]
pub extern "C" fn wasm_externtype_as_functype_const(
    et: &wasm_externtype_t,
) -> Option<&wasm_functype_t> {
    let cache = et
        .cache
        .get_or_try_init(|| -> Result<_, ()> {
            let functype = et.ty.func().ok_or(())?.clone();
            let m = wasm_functype_t::new(functype);
            Ok(Cache::Func(m))
        })
        .ok()?;

    match cache {
        Cache::Func(m) => Some(m),
        _ => None,
    }
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
    let cache = et
        .cache
        .get_or_try_init(|| -> Result<_, ()> {
            let globaltype = et.ty.global().ok_or(())?.clone();
            let m = wasm_globaltype_t::new(globaltype);
            Ok(Cache::Global(m))
        })
        .ok()?;

    match cache {
        Cache::Global(m) => Some(m),
        _ => None,
    }
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
    let cache = et
        .cache
        .get_or_try_init(|| -> Result<_, ()> {
            let tabletype = et.ty.table().ok_or(())?.clone();
            let m = wasm_tabletype_t::new(tabletype);
            Ok(Cache::Table(m))
        })
        .ok()?;

    match cache {
        Cache::Table(m) => Some(m),
        _ => None,
    }
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
    let cache = et
        .cache
        .get_or_try_init(|| -> Result<_, ()> {
            let memorytype = et.ty.memory().ok_or(())?.clone();
            let m = wasm_memorytype_t::new(memorytype);
            Ok(Cache::Memory(m))
        })
        .ok()?;

    match cache {
        Cache::Memory(m) => Some(m),
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn wasm_externtype_delete(_et: Box<wasm_externtype_t>) {}
