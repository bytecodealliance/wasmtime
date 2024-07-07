use crate::wasm_config_t;
use wasmtime::Engine;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_engine_t {
    pub(crate) engine: Engine,
}

wasmtime_c_api_macros::declare_own!(wasm_engine_t);

#[no_mangle]
pub extern "C" fn wasm_engine_new() -> Box<wasm_engine_t> {
    // Enable the `env_logger` crate since this is as good a place as any to
    // support some "top level initialization" for the C API. Almost all support
    // should go through this one way or another, so this ensures that
    // `RUST_LOG` should work reasonably well.
    //
    // Note that we `drop` the result here since this fails after the first
    // initialization attempt. We don't mind that though because this function
    // can be called multiple times, so we just ignore the result.
    #[cfg(feature = "logging")]
    drop(env_logger::try_init());

    Box::new(wasm_engine_t {
        engine: Engine::default(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_engine_new_with_config(c: Box<wasm_config_t>) -> Box<wasm_engine_t> {
    #[cfg(feature = "logging")]
    drop(env_logger::try_init());

    let config = c.config;
    Box::new(wasm_engine_t {
        engine: Engine::new(&config).unwrap(),
    })
}

#[no_mangle]
pub extern "C" fn wasmtime_engine_clone(engine: &wasm_engine_t) -> Box<wasm_engine_t> {
    Box::new(engine.clone())
}

#[no_mangle]
pub extern "C" fn wasmtime_engine_increment_epoch(engine: &wasm_engine_t) {
    engine.engine.increment_epoch();
}
