use crate::{wasm_extern_t, wasm_extern_vec_t, wasm_module_t, wasm_trap_t};
use crate::{wasm_instancetype_t, wasm_store_t, wasmtime_error_t};
use anyhow::Result;
use std::ptr;
use wasmtime::{Extern, Instance, Trap};

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_instance_t {
    ext: wasm_extern_t,
}

wasmtime_c_api_macros::declare_ref!(wasm_instance_t);

impl wasm_instance_t {
    pub(crate) fn new(instance: Instance) -> wasm_instance_t {
        wasm_instance_t {
            ext: wasm_extern_t {
                which: instance.into(),
            },
        }
    }

    pub(crate) fn try_from(e: &wasm_extern_t) -> Option<&wasm_instance_t> {
        match &e.which {
            Extern::Instance(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    pub(crate) fn instance(&self) -> &Instance {
        match &self.ext.which {
            Extern::Instance(i) => i,
            _ => unreachable!(),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_instance_new(
    store: &wasm_store_t,
    wasm_module: &wasm_module_t,
    imports: *const wasm_extern_vec_t,
    result: Option<&mut *mut wasm_trap_t>,
) -> Option<Box<wasm_instance_t>> {
    let mut instance = ptr::null_mut();
    let mut trap = ptr::null_mut();
    let err = _wasmtime_instance_new(
        store,
        wasm_module,
        (*imports).as_slice(),
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
    imports: *const wasm_extern_vec_t,
    instance_ptr: &mut *mut wasm_instance_t,
    trap_ptr: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    _wasmtime_instance_new(store, module, (*imports).as_slice(), instance_ptr, trap_ptr)
}

fn _wasmtime_instance_new(
    store: &wasm_store_t,
    module: &wasm_module_t,
    imports: &[Option<Box<wasm_extern_t>>],
    instance_ptr: &mut *mut wasm_instance_t,
    trap_ptr: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    let store = &store.store;
    let imports = imports
        .iter()
        .filter_map(|import| match import {
            Some(i) => Some(i.which.clone()),
            None => None,
        })
        .collect::<Vec<_>>();
    handle_instantiate(
        Instance::new(store, module.module(), &imports),
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
pub extern "C" fn wasm_instance_as_extern(m: &wasm_instance_t) -> &wasm_extern_t {
    &m.ext
}

#[no_mangle]
pub extern "C" fn wasm_instance_exports(instance: &wasm_instance_t, out: &mut wasm_extern_vec_t) {
    out.set_buffer(
        instance
            .instance()
            .exports()
            .map(|e| {
                Some(Box::new(wasm_extern_t {
                    which: e.into_extern(),
                }))
            })
            .collect(),
    );
}

#[no_mangle]
pub extern "C" fn wasm_instance_type(f: &wasm_instance_t) -> Box<wasm_instancetype_t> {
    Box::new(wasm_instancetype_t::new(f.instance().ty()))
}
