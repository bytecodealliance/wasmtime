use std::ffi::{c_char, CStr};

use wasmtime::component::{Func, Instance};

use crate::WasmtimeStoreContextMut;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_instance_get_func(
    instance: &Instance,
    context: WasmtimeStoreContextMut<'_>,
    name: *const c_char,
    func_out: &mut Func,
) -> bool {
    let name = unsafe { CStr::from_ptr(name) };
    let Ok(name) = name.to_str() else {
        return false;
    };

    if let Some(func) = instance.get_func(context, name) {
        *func_out = func;
        true
    } else {
        false
    }
}
