use crate::{wasm_extern_t, wasm_extern_vec_t, wasm_module_t, wasm_trap_t};
use crate::{wasm_store_t, wasmtime_error_t, ExternHost};
use anyhow::Result;
use std::cell::RefCell;
use std::ptr;
use wasmtime::{Extern, HostRef, Instance, Store, Trap};

#[repr(C)]
#[derive(Clone)]
pub struct wasm_instance_t {
    pub(crate) instance: HostRef<Instance>,
    exports_cache: RefCell<Option<Vec<ExternHost>>>,
}

wasmtime_c_api_macros::declare_ref!(wasm_instance_t);

impl wasm_instance_t {
    pub(crate) fn new(instance: Instance) -> wasm_instance_t {
        wasm_instance_t {
            instance: HostRef::new(instance),
            exports_cache: RefCell::new(None),
        }
    }

    fn anyref(&self) -> wasmtime::AnyRef {
        self.instance.anyref()
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_instance_new(
    store: &wasm_store_t,
    wasm_module: &wasm_module_t,
    imports: *const Box<wasm_extern_t>,
    result: Option<&mut *mut wasm_trap_t>,
) -> Option<Box<wasm_instance_t>> {
    let store = &store.store.borrow();
    let module = &wasm_module.module.borrow();
    if !Store::same(&store, module.store()) {
        if let Some(result) = result {
            let trap = Trap::new("wasm_store_t must match store in wasm_module_t");
            let trap = Box::new(wasm_trap_t::new(trap));
            *result = Box::into_raw(trap);
        }
        return None;
    }
    let mut instance = ptr::null_mut();
    let mut trap = ptr::null_mut();
    let err = wasmtime_instance_new(
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
    module: &wasm_module_t,
    imports: *const Box<wasm_extern_t>,
    num_imports: usize,
    instance_ptr: &mut *mut wasm_instance_t,
    trap_ptr: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    _wasmtime_instance_new(
        module,
        std::slice::from_raw_parts(imports, num_imports),
        instance_ptr,
        trap_ptr,
    )
}

fn _wasmtime_instance_new(
    module: &wasm_module_t,
    imports: &[Box<wasm_extern_t>],
    instance_ptr: &mut *mut wasm_instance_t,
    trap_ptr: &mut *mut wasm_trap_t,
) -> Option<Box<wasmtime_error_t>> {
    let imports = imports
        .iter()
        .map(|import| match &import.which {
            ExternHost::Func(e) => Extern::Func(e.borrow().clone()),
            ExternHost::Table(e) => Extern::Table(e.borrow().clone()),
            ExternHost::Global(e) => Extern::Global(e.borrow().clone()),
            ExternHost::Memory(e) => Extern::Memory(e.borrow().clone()),
        })
        .collect::<Vec<_>>();
    let module = &module.module.borrow();
    handle_instantiate(Instance::new(module, &imports), instance_ptr, trap_ptr)
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
    let mut cache = instance.exports_cache.borrow_mut();
    let exports = cache.get_or_insert_with(|| {
        let instance = &instance.instance.borrow();
        instance
            .exports()
            .iter()
            .map(|e| match e {
                Extern::Func(f) => ExternHost::Func(HostRef::new(f.clone())),
                Extern::Global(f) => ExternHost::Global(HostRef::new(f.clone())),
                Extern::Memory(f) => ExternHost::Memory(HostRef::new(f.clone())),
                Extern::Table(f) => ExternHost::Table(HostRef::new(f.clone())),
            })
            .collect()
    });
    let mut buffer = Vec::with_capacity(exports.len());
    for e in exports {
        let ext = Box::new(wasm_extern_t { which: e.clone() });
        buffer.push(Some(ext));
    }
    out.set_buffer(buffer);
}
