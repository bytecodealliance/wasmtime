use crate::{wasm_externtype_t, wasm_limits_t, CExternType};
use once_cell::unsync::OnceCell;
use wasmtime::MemoryType;

#[repr(transparent)]
#[derive(Clone)]
pub struct wasm_memorytype_t {
    ext: wasm_externtype_t,
}

wasmtime_c_api_macros::declare_ty!(wasm_memorytype_t);

#[derive(Clone)]
pub(crate) struct CMemoryType {
    pub(crate) ty: MemoryType,
    limits_cache: OnceCell<wasm_limits_t>,
}

impl wasm_memorytype_t {
    pub(crate) fn new(ty: MemoryType) -> wasm_memorytype_t {
        wasm_memorytype_t {
            ext: wasm_externtype_t::new(ty.into()),
        }
    }

    pub(crate) fn try_from(e: &wasm_externtype_t) -> Option<&wasm_memorytype_t> {
        match &e.which {
            CExternType::Memory(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    pub(crate) fn ty(&self) -> &CMemoryType {
        match &self.ext.which {
            CExternType::Memory(f) => &f,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

impl CMemoryType {
    pub(crate) fn new(ty: MemoryType) -> CMemoryType {
        CMemoryType {
            ty,
            limits_cache: OnceCell::new(),
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_memorytype_new(limits: &wasm_limits_t) -> Box<wasm_memorytype_t> {
    Box::new(wasm_memorytype_t::new(MemoryType::new(
        limits.to_wasmtime(),
    )))
}

#[no_mangle]
pub extern "C" fn wasm_memorytype_limits(mt: &wasm_memorytype_t) -> &wasm_limits_t {
    let mt = mt.ty();
    mt.limits_cache.get_or_init(|| {
        let limits = mt.ty.limits();
        wasm_limits_t {
            min: limits.min(),
            max: limits.max().unwrap_or(u32::max_value()),
        }
    })
}

#[no_mangle]
pub extern "C" fn wasm_memorytype_as_externtype(ty: &wasm_memorytype_t) -> &wasm_externtype_t {
    &ty.ext
}

#[no_mangle]
pub extern "C" fn wasm_memorytype_as_externtype_const(
    ty: &wasm_memorytype_t,
) -> &wasm_externtype_t {
    &ty.ext
}
