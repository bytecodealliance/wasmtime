use crate::{wasm_extern_t, wasm_memorytype_t, wasm_store_t, ExternHost};
use wasmtime::{HostRef, Memory};

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_memory_t {
    ext: wasm_extern_t,
}

wasmtime_c_api_macros::declare_ref!(wasm_memory_t);

pub type wasm_memory_pages_t = u32;

impl wasm_memory_t {
    pub(crate) fn try_from(e: &wasm_extern_t) -> Option<&wasm_memory_t> {
        match &e.which {
            ExternHost::Memory(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    fn memory(&self) -> &HostRef<Memory> {
        match &self.ext.which {
            ExternHost::Memory(m) => m,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    fn anyref(&self) -> wasmtime::AnyRef {
        self.memory().anyref()
    }
}

#[no_mangle]
pub extern "C" fn wasm_memory_new(
    store: &wasm_store_t,
    mt: &wasm_memorytype_t,
) -> Box<wasm_memory_t> {
    let memory = HostRef::new(Memory::new(&store.store.borrow(), mt.ty().ty.clone()));
    Box::new(wasm_memory_t {
        ext: wasm_extern_t {
            which: ExternHost::Memory(memory),
        },
    })
}

#[no_mangle]
pub extern "C" fn wasm_memory_as_extern(m: &wasm_memory_t) -> &wasm_extern_t {
    &m.ext
}

#[no_mangle]
pub extern "C" fn wasm_memory_type(m: &wasm_memory_t) -> Box<wasm_memorytype_t> {
    let ty = m.memory().borrow().ty().clone();
    Box::new(wasm_memorytype_t::new(ty))
}

#[no_mangle]
pub extern "C" fn wasm_memory_data(m: &wasm_memory_t) -> *mut u8 {
    m.memory().borrow().data_ptr()
}

#[no_mangle]
pub extern "C" fn wasm_memory_data_size(m: &wasm_memory_t) -> usize {
    m.memory().borrow().data_size()
}

#[no_mangle]
pub extern "C" fn wasm_memory_size(m: &wasm_memory_t) -> wasm_memory_pages_t {
    m.memory().borrow().size()
}

#[no_mangle]
pub extern "C" fn wasm_memory_grow(m: &wasm_memory_t, delta: wasm_memory_pages_t) -> bool {
    m.memory().borrow().grow(delta).is_ok()
}
