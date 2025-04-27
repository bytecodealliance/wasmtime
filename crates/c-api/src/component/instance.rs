use wasmtime::component::{Func, Instance};

use crate::{wasm_name_t, WasmtimeStoreContextMut};

use super::wasmtime_component_export_index_t;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_instance_get_export_index(
    instance: &Instance,
    context: WasmtimeStoreContextMut<'_>,
    instance_export_index: *const wasmtime_component_export_index_t,
    name: &mut wasm_name_t,
    out: &mut *mut wasmtime_component_export_index_t,
) -> bool {
    let name = name.take();
    let Ok(name) = String::from_utf8(name) else {
        return false;
    };

    let instance_export_index = if instance_export_index.is_null() {
        None
    } else {
        Some((*instance_export_index).export_index)
    };

    let export_index = instance.get_export_index(context, instance_export_index.as_ref(), &name);

    if let Some(export_index) = export_index {
        *out = Box::into_raw(Box::new(wasmtime_component_export_index_t { export_index }));
        true
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_instance_get_func(
    instance: &Instance,
    context: WasmtimeStoreContextMut<'_>,
    export_index: &wasmtime_component_export_index_t,
    func_out: &mut Func,
) -> bool {
    if let Some(func) = instance.get_func(context, export_index.export_index) {
        *func_out = func;
        true
    } else {
        false
    }
}
