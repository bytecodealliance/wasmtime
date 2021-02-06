use crate::wasm_engine_t;
use wasmtime::{InterruptHandle, Store};

#[repr(C)]
#[derive(Clone)]
pub struct wasm_store_t {
    pub(crate) store: Store,
}

wasmtime_c_api_macros::declare_own!(wasm_store_t);

#[no_mangle]
pub extern "C" fn wasm_store_new(engine: &wasm_engine_t) -> Box<wasm_store_t> {
    let engine = &engine.engine;
    Box::new(wasm_store_t {
        store: Store::new(&engine),
    })
}

#[no_mangle]
pub extern "C" fn wasmtime_store_gc(store: &wasm_store_t) {
    store.store.gc();
}

#[repr(C)]
pub struct wasmtime_interrupt_handle_t {
    handle: InterruptHandle,
}

wasmtime_c_api_macros::declare_own!(wasmtime_interrupt_handle_t);

#[no_mangle]
pub extern "C" fn wasmtime_interrupt_handle_new(
    store: &wasm_store_t,
) -> Option<Box<wasmtime_interrupt_handle_t>> {
    Some(Box::new(wasmtime_interrupt_handle_t {
        handle: store.store.interrupt_handle().ok()?,
    }))
}

#[no_mangle]
pub extern "C" fn wasmtime_interrupt_handle_interrupt(handle: &wasmtime_interrupt_handle_t) {
    handle.handle.interrupt();
}

#[no_mangle]
pub extern "C" fn wasmtime_add_fuel(store: &wasm_store_t, fuel: u64) {
    store.store.add_fuel(fuel);
}

#[no_mangle]
pub extern "C" fn wasmtime_fuel_consumed(store: &wasm_store_t) -> u64 {
    store.store.fuel_consumed().unwrap_or(0)
}
