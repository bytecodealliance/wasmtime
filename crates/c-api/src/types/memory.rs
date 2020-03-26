use crate::wasm_limits_t;
use once_cell::unsync::OnceCell;
use wasmtime::MemoryType;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_memorytype_t {
    pub(crate) memorytype: MemoryType,
    limits_cache: OnceCell<wasm_limits_t>,
}

impl wasm_memorytype_t {
    pub(crate) fn new(memorytype: MemoryType) -> wasm_memorytype_t {
        wasm_memorytype_t {
            memorytype,
            limits_cache: OnceCell::new(),
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_memorytype_new(limits: &wasm_limits_t) -> Box<wasm_memorytype_t> {
    Box::new(wasm_memorytype_t {
        memorytype: MemoryType::new(limits.to_wasmtime()),
        limits_cache: OnceCell::new(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_memorytype_limits(mt: &wasm_memorytype_t) -> &wasm_limits_t {
    mt.limits_cache.get_or_init(|| {
        let limits = mt.memorytype.limits();
        wasm_limits_t {
            min: limits.min(),
            max: limits.max().unwrap_or(u32::max_value()),
        }
    })
}

#[no_mangle]
pub extern "C" fn wasm_memorytype_delete(_mt: Box<wasm_memorytype_t>) {}
