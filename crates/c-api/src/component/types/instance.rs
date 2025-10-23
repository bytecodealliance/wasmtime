use crate::{wasm_engine_t, wasmtime_component_item_t};
use std::mem::MaybeUninit;
use wasmtime::component::types::ComponentInstance;

type_wrapper! {
    pub struct wasmtime_component_instance_type_t {
        pub(crate) ty: ComponentInstance,
    }

    clone: wasmtime_component_instance_type_clone,
    delete: wasmtime_component_instance_type_delete,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_instance_type_export_count(
    ty: &wasmtime_component_instance_type_t,
    engine: &wasm_engine_t,
) -> usize {
    ty.ty.exports(&engine.engine).count()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_instance_type_export_get(
    ty: &wasmtime_component_instance_type_t,
    engine: &wasm_engine_t,
    name: *const u8,
    name_len: usize,
    ret: &mut MaybeUninit<wasmtime_component_item_t>,
) -> bool {
    let name = unsafe { std::slice::from_raw_parts(name, name_len) };
    let Ok(name) = std::str::from_utf8(name) else {
        return false;
    };
    match ty.ty.get_export(&engine.engine, name) {
        Some(item) => {
            ret.write(item.into());
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_instance_type_export_nth(
    ty: &wasmtime_component_instance_type_t,
    engine: &wasm_engine_t,
    nth: usize,
    name_ret: &mut MaybeUninit<*const u8>,
    name_len_ret: &mut MaybeUninit<usize>,
    type_ret: &mut MaybeUninit<wasmtime_component_item_t>,
) -> bool {
    match ty.ty.exports(&engine.engine).nth(nth) {
        Some((name, item)) => {
            let name: &str = name;
            name_ret.write(name.as_ptr());
            name_len_ret.write(name.len());
            type_ret.write(item.into());
            true
        }
        None => false,
    }
}
