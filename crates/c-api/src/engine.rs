use crate::wasm_config_t;
use wasmtime::{Engine, HostRef};

#[repr(C)]
#[derive(Clone)]
pub struct wasm_engine_t {
    pub(crate) engine: HostRef<Engine>,
}

wasmtime_c_api_macros::declare_own!(wasm_engine_t);

#[no_mangle]
pub extern "C" fn wasm_engine_new() -> Box<wasm_engine_t> {
    Box::new(wasm_engine_t {
        engine: HostRef::new(Engine::default()),
    })
}

#[no_mangle]
pub extern "C" fn wasm_engine_new_with_config(c: Box<wasm_config_t>) -> Box<wasm_engine_t> {
    let config = c.config;
    Box::new(wasm_engine_t {
        engine: HostRef::new(Engine::new(&config)),
    })
}
