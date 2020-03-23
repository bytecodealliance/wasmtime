//! This file defines the extern "C" API extension, which are specific
//! to the wasmtime implementation.

use crate::*;
use std::str;
use wasmtime::{Extern, Linker, OptLevel, ProfilingStrategy, Strategy};

#[repr(u8)]
#[derive(Clone)]
pub enum wasmtime_strategy_t {
    WASMTIME_STRATEGY_AUTO,
    WASMTIME_STRATEGY_CRANELIFT,
    WASMTIME_STRATEGY_LIGHTBEAM,
}

#[repr(u8)]
#[derive(Clone)]
pub enum wasmtime_opt_level_t {
    WASMTIME_OPT_LEVEL_NONE,
    WASMTIME_OPT_LEVEL_SPEED,
    WASMTIME_OPT_LEVEL_SPEED_AND_SIZE,
}

#[repr(u8)]
#[derive(Clone)]
pub enum wasmtime_profiling_strategy_t {
    WASMTIME_PROFILING_STRATEGY_NONE,
    WASMTIME_PROFILING_STRATEGY_JITDUMP,
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_config_debug_info_set(c: *mut wasm_config_t, enable: bool) {
    (*c).config.debug_info(enable);
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_config_wasm_threads_set(c: *mut wasm_config_t, enable: bool) {
    (*c).config.wasm_threads(enable);
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_config_wasm_reference_types_set(
    c: *mut wasm_config_t,
    enable: bool,
) {
    (*c).config.wasm_reference_types(enable);
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_config_wasm_simd_set(c: *mut wasm_config_t, enable: bool) {
    (*c).config.wasm_simd(enable);
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_config_wasm_bulk_memory_set(c: *mut wasm_config_t, enable: bool) {
    (*c).config.wasm_bulk_memory(enable);
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_config_wasm_multi_value_set(c: *mut wasm_config_t, enable: bool) {
    (*c).config.wasm_multi_value(enable);
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_config_strategy_set(
    c: *mut wasm_config_t,
    strategy: wasmtime_strategy_t,
) {
    use wasmtime_strategy_t::*;
    drop((*c).config.strategy(match strategy {
        WASMTIME_STRATEGY_AUTO => Strategy::Auto,
        WASMTIME_STRATEGY_CRANELIFT => Strategy::Cranelift,
        WASMTIME_STRATEGY_LIGHTBEAM => Strategy::Lightbeam,
    }));
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_config_cranelift_debug_verifier_set(
    c: *mut wasm_config_t,
    enable: bool,
) {
    (*c).config.cranelift_debug_verifier(enable);
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_config_cranelift_opt_level_set(
    c: *mut wasm_config_t,
    opt_level: wasmtime_opt_level_t,
) {
    use wasmtime_opt_level_t::*;
    (*c).config.cranelift_opt_level(match opt_level {
        WASMTIME_OPT_LEVEL_NONE => OptLevel::None,
        WASMTIME_OPT_LEVEL_SPEED => OptLevel::Speed,
        WASMTIME_OPT_LEVEL_SPEED_AND_SIZE => OptLevel::SpeedAndSize,
    });
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_config_profiler_set(
    c: *mut wasm_config_t,
    strategy: wasmtime_profiling_strategy_t,
) {
    use wasmtime_profiling_strategy_t::*;
    drop((*c).config.profiler(match strategy {
        WASMTIME_PROFILING_STRATEGY_NONE => ProfilingStrategy::None,
        WASMTIME_PROFILING_STRATEGY_JITDUMP => ProfilingStrategy::JitDump,
    }));
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_wat2wasm(
    _engine: *mut wasm_engine_t,
    wat: *const wasm_byte_vec_t,
    ret: *mut wasm_byte_vec_t,
    error: *mut wasm_byte_vec_t,
) -> bool {
    let wat = match str::from_utf8((*wat).as_slice()) {
        Ok(s) => s,
        Err(_) => {
            if !error.is_null() {
                (*error).set_from_slice(b"input was not valid utf-8");
            }
            return false;
        }
    };
    match wat::parse_str(wat) {
        Ok(bytes) => {
            (*ret).set_from_slice(&bytes);
            true
        }
        Err(e) => {
            if !error.is_null() {
                (*error).set_from_slice(e.to_string().as_bytes());
            }
            false
        }
    }
}

#[repr(C)]
pub struct wasmtime_linker_t {
    linker: Linker,
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_linker_new(store: *mut wasm_store_t) -> *mut wasmtime_linker_t {
    Box::into_raw(Box::new(wasmtime_linker_t {
        linker: Linker::new(&(*store).store.borrow()),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_linker_delete(linker: *mut wasmtime_linker_t) {
    drop(Box::from_raw(linker));
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_linker_define(
    linker: *mut wasmtime_linker_t,
    module: *const wasm_name_t,
    name: *const wasm_name_t,
    item: *const wasm_extern_t,
) -> bool {
    let linker = &mut (*linker).linker;
    let module = match str::from_utf8((*module).as_slice()) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let name = match str::from_utf8((*name).as_slice()) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let item = match &(*item).which {
        ExternHost::Func(e) => Extern::Func(e.borrow().clone()),
        ExternHost::Table(e) => Extern::Table(e.borrow().clone()),
        ExternHost::Global(e) => Extern::Global(e.borrow().clone()),
        ExternHost::Memory(e) => Extern::Memory(e.borrow().clone()),
    };
    linker.define(module, name, item).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_linker_define_wasi(
    linker: *mut wasmtime_linker_t,
    instance: *const wasi_instance_t,
) -> bool {
    let linker = &mut (*linker).linker;
    (*instance).wasi.add_to_linker(linker).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_linker_define_instance(
    linker: *mut wasmtime_linker_t,
    name: *const wasm_name_t,
    instance: *const wasm_instance_t,
) -> bool {
    let linker = &mut (*linker).linker;
    let name = match str::from_utf8((*name).as_slice()) {
        Ok(s) => s,
        Err(_) => return false,
    };
    linker
        .instance(name, &(*instance).instance.borrow())
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_linker_instantiate(
    linker: *const wasmtime_linker_t,
    module: *const wasm_module_t,
    trap: *mut *mut wasm_trap_t,
) -> *mut wasm_instance_t {
    let linker = &(*linker).linker;
    handle_instantiate(linker.instantiate(&(*module).module.borrow()), trap)
}
