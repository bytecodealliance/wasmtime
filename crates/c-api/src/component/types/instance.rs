use crate::{wasm_engine_t, wasmtime_component_extern_t};
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
pub unsafe extern "C" fn wasmtime_component_instance_type_export_get<'a>(
    ty: &'a wasmtime_component_instance_type_t,
    engine: &'a wasm_engine_t,
    name: *const u8,
    name_len: usize,
    ret: &mut MaybeUninit<Box<wasmtime_component_extern_t<'a>>>,
) -> bool {
    let name = unsafe { std::slice::from_raw_parts(name, name_len) };
    let Ok(name) = std::str::from_utf8(name) else {
        return false;
    };
    match ty.ty.get_export(&engine.engine, name) {
        Some(e) => {
            ret.write(Box::new(e.into()));
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_instance_type_export_nth<'a>(
    ty: &'a wasmtime_component_instance_type_t,
    engine: &'a wasm_engine_t,
    nth: usize,
    name_ret: &mut MaybeUninit<*const u8>,
    name_len_ret: &mut MaybeUninit<usize>,
    ret: &mut MaybeUninit<Box<wasmtime_component_extern_t<'a>>>,
) -> bool {
    match ty.ty.exports(&engine.engine).nth(nth) {
        Some((name, e)) => {
            let name: &str = name;
            name_ret.write(name.as_ptr());
            name_len_ret.write(name.len());
            ret.write(Box::new(e.into()));
            true
        }
        None => false,
    }
}
