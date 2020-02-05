//! This file defines the extern "C" API extension, which are specific
//! to the wasmtime implementation.

use crate::wasm_config_t;
use wasmtime::{OptLevel, Strategy};

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
