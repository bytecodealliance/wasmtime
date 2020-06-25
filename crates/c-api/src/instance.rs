use crate::{wasm_extern_t, wasm_extern_vec_t, wasm_module_t, wasm_trap_t};
use crate::{wasm_store_t, wasmtime_error_t};
use anyhow::Result;
use std::ptr;
use wasmtime::{Instance, Trap};

#[repr(C)]
#[derive(Clone)]
pub struct wasm_instance_t {
    pub(crate) instance: Instance,
}

wasmtime_c_api_macros::declare_ref!(wasm_instance_t);

impl wasm_instance_t {
    pub(crate) fn new(instance: Instance) -> wasm_instance_t {
        wasm_instance_t { instance: instance }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_instance_new(
    store: &wasm_store_t,
    wasm_module: &wasm_module_t,
    imports: *const Box<wasm_extern_t>,
    result: Option<&mut *mut wasm_trap_t>,
) -> Option<Box<wasm_instance_t>> {
    let mut instance = ptr::null_mut();
    let mut trap = ptr::null_mut();
    let err = wasmtime_instance_new(
        store,
        wasm_module,
        imports,
        wasm_module.imports.len(),
        &mut instance,
        &mut trap,
    );
    match err {
        Some(err) => {
            assert!(trap.is_null());
            assert!(instance.is_null());
            if let Some(result) = result {
                *result = Box::into_raw(err.to_trap());
            }
            None
        }
        None => {
            if instance.is_null() {
                assert!(!trap.is_null());
                if let Some(result) = result {
                    *result = trap;
                } else {
                    drop(Box::from_raw(trap))
                }
                None
            } else {
                assert!(trap.is_null());
                Some(Box::from_raw(instance))
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_instance_new(
    store: &wasm_store_t,
    module: &wasm_module_t,
    imports: *const Box<wasm_extern_t>,
    num_imports: usize,
    instance_ptr: &mut *mut wasm_instance_t,
    trap_ptr: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    _wasmtime_instance_new(
        store,
        module,
        std::slice::from_raw_parts(imports, num_imports),
        instance_ptr,
        trap_ptr,
    )
}

fn _wasmtime_instance_new(
    store: &wasm_store_t,
    module: &wasm_module_t,
    imports: &[Box<wasm_extern_t>],
    instance_ptr: &mut *mut wasm_instance_t,
    trap_ptr: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    let store = &store.store;
    let imports = imports
        .iter()
        .map(|import| import.which.clone())
        .collect::<Vec<_>>();
    handle_instantiate(
        Instance::new(store, &module.module, &imports),
        instance_ptr,
        trap_ptr,
    )
}

pub fn handle_instantiate(
    instance: Result<Instance>,
    instance_ptr: &mut *mut wasm_instance_t,
    trap_ptr: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    fn write<T>(ptr: &mut *mut T, val: T) {
        *ptr = Box::into_raw(Box::new(val))
    }

    match instance {
        Ok(instance) => {
            write(instance_ptr, wasm_instance_t::new(instance));
            None
        }
        Err(e) => match e.downcast::<Trap>() {
            Ok(trap) => {
                write(trap_ptr, wasm_trap_t::new(trap));
                None
            }
            Err(e) => Some(Box::new(e.into())),
        },
    }
}

#[no_mangle]
pub extern "C" fn wasm_instance_exports(instance: &wasm_instance_t, out: &mut wasm_extern_vec_t) {
    out.set_buffer(
        instance
            .instance
            .exports()
            .map(|e| {
                Some(Box::new(wasm_extern_t {
                    which: e.into_extern(),
                }))
            })
            .collect(),
    );
}
