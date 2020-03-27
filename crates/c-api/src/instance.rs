use crate::{wasm_extern_t, wasm_extern_vec_t, wasm_module_t, wasm_trap_t};
use crate::{wasm_store_t, ExternHost};
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
    fn new(instance: Instance) -> wasm_instance_t {
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
    module: &wasm_module_t,
    imports: *const Box<wasm_extern_t>,
    result: Option<&mut *mut wasm_trap_t>,
) -> Option<Box<wasm_instance_t>> {
    let mut externs: Vec<Extern> = Vec::with_capacity((*module).imports.len());
    for i in 0..(*module).imports.len() {
        let import = &*imports.add(i);
        externs.push(match &import.which {
            ExternHost::Func(e) => Extern::Func(e.borrow().clone()),
            ExternHost::Table(e) => Extern::Table(e.borrow().clone()),
            ExternHost::Global(e) => Extern::Global(e.borrow().clone()),
            ExternHost::Memory(e) => Extern::Memory(e.borrow().clone()),
        });
    }
    let store = &(*store).store.borrow();
    let module = &(*module).module.borrow();
    if !Store::same(&store, module.store()) {
        if let Some(result) = result {
            let trap = Trap::new("wasm_store_t must match store in wasm_module_t");
            let trap = Box::new(wasm_trap_t {
                trap: HostRef::new(trap),
            });
            *result = Box::into_raw(trap);
        }
        return None;
    }
    handle_instantiate(Instance::new(module, &externs), result)
}

pub fn handle_instantiate(
    instance: Result<Instance>,
    result: Option<&mut *mut wasm_trap_t>,
) -> Option<Box<wasm_instance_t>> {
    match instance {
        Ok(instance) => {
            if let Some(result) = result {
                *result = ptr::null_mut();
            }
            Some(Box::new(wasm_instance_t::new(instance)))
        }
        Err(trap) => {
            if let Some(result) = result {
                let trap = match trap.downcast::<Trap>() {
                    Ok(trap) => trap,
                    Err(e) => Trap::new(format!("{:?}", e)),
                };
                let trap = Box::new(wasm_trap_t {
                    trap: HostRef::new(trap),
                });
                *result = Box::into_raw(trap);
            }
            None
        }
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
