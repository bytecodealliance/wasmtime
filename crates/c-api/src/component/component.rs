use std::ffi::{c_char, CStr};

use anyhow::Context;
use wasmtime::component::Component;

use crate::{wasm_byte_vec_t, wasm_config_t, wasm_engine_t, wasmtime_error_t};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_config_component_model_set(c: &mut wasm_config_t, enable: bool) {
    c.config.wasm_component_model(enable);
}

#[derive(Clone)]
#[repr(transparent)]
pub struct wasmtime_component_t {
    pub(crate) component: Component,
}

#[unsafe(no_mangle)]
#[cfg(any(feature = "cranelift", feature = "winch"))]
pub unsafe extern "C" fn wasmtime_component_new(
    engine: &wasm_engine_t,
    buf: *const u8,
    len: usize,
    component_out: &mut *mut wasmtime_component_t,
) -> Option<Box<wasmtime_error_t>> {
    let binary = unsafe { crate::slice_from_raw_parts(buf, len) };
    crate::handle_result(
        Component::from_binary(&engine.engine, binary),
        |component| {
            *component_out = Box::into_raw(Box::new(wasmtime_component_t { component }));
        },
    )
}

#[unsafe(no_mangle)]
#[cfg(any(feature = "cranelift", feature = "winch"))]
pub unsafe extern "C" fn wasmtime_component_serialize(
    component: &wasmtime_component_t,
    ret: &mut wasm_byte_vec_t,
) -> Option<Box<wasmtime_error_t>> {
    crate::handle_result(component.component.serialize(), |buffer| {
        ret.set_buffer(buffer);
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_deserialize(
    engine: &wasm_engine_t,
    buf: *const u8,
    len: usize,
    component_out: &mut *mut wasmtime_component_t,
) -> Option<Box<wasmtime_error_t>> {
    let binary = unsafe { crate::slice_from_raw_parts(buf, len) };
    crate::handle_result(
        unsafe { Component::deserialize(&engine.engine, binary) },
        |component| {
            *component_out = Box::into_raw(Box::new(wasmtime_component_t { component }));
        },
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_deserialize_file(
    engine: &wasm_engine_t,
    path: *const c_char,
    component_out: &mut *mut wasmtime_component_t,
) -> Option<Box<wasmtime_error_t>> {
    let path = unsafe { CStr::from_ptr(path) };
    let result = path
        .to_str()
        .context("input path is not valid utf-8")
        .and_then(|path| unsafe { Component::deserialize_file(&engine.engine, path) });
    crate::handle_result(result, |component| {
        *component_out = Box::into_raw(Box::new(wasmtime_component_t { component }));
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_clone(
    component: &wasmtime_component_t,
) -> Box<wasmtime_component_t> {
    Box::new(component.clone())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_delete(_component: Box<wasmtime_component_t>) {}
