use crate::wasm_engine_t;
use wasmtime::{HostRef, Store};

#[repr(C)]
#[derive(Clone)]
pub struct wasm_store_t {
    pub(crate) store: HostRef<Store>,
}

wasmtime_c_api_macros::declare_own!(wasm_store_t);

#[no_mangle]
pub extern "C" fn wasm_store_new(engine: &wasm_engine_t) -> Box<wasm_store_t> {
    let engine = &engine.engine;
    Box::new(wasm_store_t {
        store: HostRef::new(Store::new(&engine.borrow())),
    })
}
