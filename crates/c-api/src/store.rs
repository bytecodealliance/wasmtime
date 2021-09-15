use crate::{wasm_engine_t, wasmtime_error_t, wasmtime_val_t, ForeignData};
use std::cell::UnsafeCell;
use std::ffi::c_void;
use std::sync::Arc;
use wasmtime::{AsContext, AsContextMut, InterruptHandle, Store, StoreContext, StoreContextMut};

/// This representation of a `Store` is used to implement the `wasm.h` API.
///
/// This is stored alongside `Func` and such for `wasm.h` so each object is
/// independently owned. The usage of `Arc` here is mostly to just get it to be
/// safe to drop across multiple threads, but otherwise acquiring the `context`
/// values from this struct is considered unsafe due to it being unknown how the
/// aliasing is working on the C side of things.
///
/// The aliasing requirements are documented in the C API `wasm.h` itself (at
/// least Wasmtime's implementation).
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

/// Representation of a `Store` for `wasmtime.h` This notably tries to move more
/// burden of aliasing on the caller rather than internally, allowing for a more
/// raw representation of contexts and such that requires less `unsafe` in the
/// implementation.
///
/// Note that this notably carries `StoreData` as a payload which allows storing
/// foreign data and configuring WASI as well.
#[repr(C)]
pub struct wasmtime_store_t {
    pub(crate) store: Store<StoreData>,
}

pub type CStoreContext<'a> = StoreContext<'a, StoreData>;
pub type CStoreContextMut<'a> = StoreContextMut<'a, StoreData>;

pub struct StoreData {
    foreign: crate::ForeignData,
    #[cfg(feature = "wasi")]
    pub(crate) wasi: Option<wasmtime_wasi::WasiCtx>,

    /// Temporary storage for usage during a wasm->host call to store values
    /// in a slice we pass to the C API.
    pub hostcall_val_storage: Vec<wasmtime_val_t>,
}

#[no_mangle]
pub extern "C" fn wasmtime_store_delete(_: Box<wasmtime_store_t>) {}

#[no_mangle]
pub extern "C" fn wasmtime_store_new(
    engine: &wasm_engine_t,
    data: *mut c_void,
    finalizer: Option<extern "C" fn(*mut c_void)>,
) -> Box<wasmtime_store_t> {
    Box::new(wasmtime_store_t {
        store: Store::new(
            &engine.engine,
            StoreData {
                foreign: ForeignData { data, finalizer },
                #[cfg(feature = "wasi")]
                wasi: None,
                hostcall_val_storage: Vec::new(),
            },
        ),
    })
}

#[no_mangle]
pub extern "C" fn wasmtime_store_context(store: &mut wasmtime_store_t) -> CStoreContextMut<'_> {
    store.store.as_context_mut()
}

#[no_mangle]
pub extern "C" fn wasmtime_context_get_data(store: CStoreContext<'_>) -> *mut c_void {
    store.data().foreign.data
}

#[no_mangle]
pub extern "C" fn wasmtime_context_set_data(mut store: CStoreContextMut<'_>, data: *mut c_void) {
    store.data_mut().foreign.data = data;
}

#[cfg(feature = "wasi")]
#[no_mangle]
pub extern "C" fn wasmtime_context_set_wasi(
    mut context: CStoreContextMut<'_>,
    wasi: Box<crate::wasi_config_t>,
) -> Option<Box<wasmtime_error_t>> {
    crate::handle_result(wasi.into_wasi_ctx(), |wasi| {
        context.data_mut().wasi = Some(wasi);
    })
}

#[no_mangle]
pub extern "C" fn wasmtime_context_gc(mut context: CStoreContextMut<'_>) {
    context.gc();
}

#[no_mangle]
pub extern "C" fn wasmtime_context_add_fuel(
    mut store: CStoreContextMut<'_>,
    fuel: u64,
) -> Option<Box<wasmtime_error_t>> {
    crate::handle_result(store.add_fuel(fuel), |()| {})
}

#[no_mangle]
pub extern "C" fn wasmtime_context_fuel_consumed(store: CStoreContext<'_>, fuel: &mut u64) -> bool {
    match store.fuel_consumed() {
        Some(amt) => {
            *fuel = amt;
            true
        }
        None => false,
    }
}

#[no_mangle]
pub extern "C" fn wasmtime_context_consume_fuel(
    mut store: CStoreContextMut<'_>,
    fuel: u64,
    remaining_fuel: &mut u64,
) -> Option<Box<wasmtime_error_t>> {
    crate::handle_result(store.consume_fuel(fuel), |remaining| {
        *remaining_fuel = remaining;
    })
}

#[repr(C)]
pub struct wasmtime_interrupt_handle_t {
    handle: InterruptHandle,
}

#[no_mangle]
pub extern "C" fn wasmtime_interrupt_handle_new(
    store: CStoreContext<'_>,
) -> Option<Box<wasmtime_interrupt_handle_t>> {
    Some(Box::new(wasmtime_interrupt_handle_t {
        handle: store.interrupt_handle().ok()?,
    }))
}

#[no_mangle]
pub extern "C" fn wasmtime_interrupt_handle_interrupt(handle: &wasmtime_interrupt_handle_t) {
    handle.handle.interrupt();
}

#[no_mangle]
pub extern "C" fn wasmtime_interrupt_handle_delete(_: Box<wasmtime_interrupt_handle_t>) {}
