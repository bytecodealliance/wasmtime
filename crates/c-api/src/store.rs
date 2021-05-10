use crate::wasm_engine_t;
use std::cell::UnsafeCell;
use std::sync::Arc;
use wasmtime::{AsContext, AsContextMut, Store, StoreContext, StoreContextMut};

#[derive(Clone)]
pub struct StoreRef {
    store: Arc<UnsafeCell<Store<()>>>,
}

impl StoreRef {
    pub unsafe fn context(&self) -> StoreContext<'_, ()> {
        (*self.store.get()).as_context()
    }

    pub unsafe fn context_mut(&mut self) -> StoreContextMut<'_, ()> {
        (*self.store.get()).as_context_mut()
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct wasm_store_t {
    pub(crate) store: StoreRef,
}

wasmtime_c_api_macros::declare_own!(wasm_store_t);

#[no_mangle]
pub extern "C" fn wasm_store_new(engine: &wasm_engine_t) -> Box<wasm_store_t> {
    let engine = &engine.engine;
    let store = Store::new(engine, ());
    Box::new(wasm_store_t {
        store: StoreRef {
            store: Arc::new(UnsafeCell::new(store)),
        },
    })
}

// #[no_mangle]
// pub extern "C" fn wasmtime_store_gc(store: &wasm_store_t) {
//     store.store.gc();
// }

// #[repr(C)]
// pub struct wasmtime_interrupt_handle_t {
//     handle: InterruptHandle,
// }

// wasmtime_c_api_macros::declare_own!(wasmtime_interrupt_handle_t);

// #[no_mangle]
// pub extern "C" fn wasmtime_interrupt_handle_new(
//     store: &wasm_store_t,
// ) -> Option<Box<wasmtime_interrupt_handle_t>> {
//     Some(Box::new(wasmtime_interrupt_handle_t {
//         handle: store.store.interrupt_handle().ok()?,
//     }))
// }

// #[no_mangle]
// pub extern "C" fn wasmtime_interrupt_handle_interrupt(handle: &wasmtime_interrupt_handle_t) {
//     handle.handle.interrupt();
// }

// #[no_mangle]
// pub extern "C" fn wasmtime_store_add_fuel(
//     store: &wasm_store_t,
//     fuel: u64,
// ) -> Option<Box<wasmtime_error_t>> {
//     crate::handle_result(store.store.add_fuel(fuel), |()| {})
// }

// #[no_mangle]
// pub extern "C" fn wasmtime_store_fuel_consumed(store: &wasm_store_t, fuel: &mut u64) -> bool {
//     match store.store.fuel_consumed() {
//         Some(amt) => {
//             *fuel = amt;
//             true
//         }
//         None => false,
//     }
// }
