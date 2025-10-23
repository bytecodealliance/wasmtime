use crate::{
    wasm_engine_t, wasm_functype_t, wasmtime_component_func_type_t,
    wasmtime_component_instance_type_t, wasmtime_component_resource_type_t,
    wasmtime_component_valtype_t, wasmtime_module_type_t,
};
use std::mem::{ManuallyDrop, MaybeUninit};
use wasmtime::component::types::{Component, ComponentItem};

type_wrapper! {
    pub struct wasmtime_component_type_t {
        pub(crate) ty: Component,
    }

    clone: wasmtime_component_type_clone,
    delete: wasmtime_component_type_delete,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_type_import_count(
    ty: &wasmtime_component_type_t,
    engine: &wasm_engine_t,
) -> usize {
    ty.ty.imports(&engine.engine).len()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_type_import_get(
    ty: &wasmtime_component_type_t,
    engine: &wasm_engine_t,
    name: *const u8,
    name_len: usize,
    ret: &mut MaybeUninit<wasmtime_component_item_t>,
) -> bool {
    let name = unsafe { std::slice::from_raw_parts(name, name_len) };
    let Ok(name) = std::str::from_utf8(name) else {
        return false;
    };
    match ty.ty.get_import(&engine.engine, name) {
        Some(item) => {
            ret.write(item.into());
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_type_import_nth(
    ty: &wasmtime_component_type_t,
    engine: &wasm_engine_t,
    nth: usize,
    name_ret: &mut MaybeUninit<*const u8>,
    name_len_ret: &mut MaybeUninit<usize>,
    type_ret: &mut MaybeUninit<wasmtime_component_item_t>,
) -> bool {
    match ty.ty.imports(&engine.engine).nth(nth) {
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

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_type_export_count(
    ty: &wasmtime_component_type_t,
    engine: &wasm_engine_t,
) -> usize {
    ty.ty.exports(&engine.engine).len()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_type_export_get(
    ty: &wasmtime_component_type_t,
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
pub extern "C" fn wasmtime_component_type_export_nth(
    ty: &wasmtime_component_type_t,
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

#[derive(Clone)]
#[repr(C, u8)]
pub enum wasmtime_component_item_t {
    Component(Box<wasmtime_component_type_t>),
    ComponentInstance(Box<wasmtime_component_instance_type_t>),
    Module(Box<wasmtime_module_type_t>),
    ComponentFunc(Box<wasmtime_component_func_type_t>),
    Resource(Box<wasmtime_component_resource_type_t>),
    CoreFunc(Box<wasm_functype_t>),
    Type(wasmtime_component_valtype_t),
}

impl From<ComponentItem> for wasmtime_component_item_t {
    fn from(item: ComponentItem) -> Self {
        match item {
            ComponentItem::Component(ty) => {
                wasmtime_component_item_t::Component(Box::new(ty.into()))
            }
            ComponentItem::ComponentInstance(ty) => {
                wasmtime_component_item_t::ComponentInstance(Box::new(ty.into()))
            }
            ComponentItem::Module(ty) => wasmtime_component_item_t::Module(Box::new(ty.into())),
            ComponentItem::ComponentFunc(ty) => {
                wasmtime_component_item_t::ComponentFunc(Box::new(ty.into()))
            }
            ComponentItem::Resource(ty) => wasmtime_component_item_t::Resource(Box::new(ty.into())),
            ComponentItem::CoreFunc(ty) => {
                wasmtime_component_item_t::CoreFunc(Box::new(wasm_functype_t::new(ty)))
            }
            ComponentItem::Type(ty) => wasmtime_component_item_t::Type(ty.into()),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_item_clone(
    item: &wasmtime_component_item_t,
    ret: &mut MaybeUninit<wasmtime_component_item_t>,
) {
    ret.write(item.clone());
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_item_delete(
    item: &mut ManuallyDrop<wasmtime_component_item_t>,
) {
    unsafe {
        ManuallyDrop::drop(item);
    }
}
