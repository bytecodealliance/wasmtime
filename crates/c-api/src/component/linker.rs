use wasmtime::component::{Instance, Linker};

use crate::{wasm_engine_t, wasmtime_error_t, WasmtimeStoreContextMut, WasmtimeStoreData};

use super::wasmtime_component_t;

#[repr(transparent)]
pub struct wasmtime_component_linker_t {
    pub(crate) linker: Linker<WasmtimeStoreData>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_linker_new(
    engine: &wasm_engine_t,
) -> Box<wasmtime_component_linker_t> {
    Box::new(wasmtime_component_linker_t {
        linker: Linker::new(&engine.engine),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_linker_instantiate(
    linker: &wasmtime_component_linker_t,
    context: WasmtimeStoreContextMut<'_>,
    component: &wasmtime_component_t,
    instance_out: &mut Instance,
) -> Option<Box<wasmtime_error_t>> {
    let result = linker.linker.instantiate(context, &component.component);
    crate::handle_result(result, |instance| *instance_out = instance)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_linker_delete(
    _linker: Box<wasmtime_component_linker_t>,
) {
}
