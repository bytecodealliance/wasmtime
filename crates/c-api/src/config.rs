// Don't worry about unused imports if we're frobbing features, only worry about
// them with the default set of features enabled.
#![cfg_attr(not(feature = "cache"), allow(unused_imports))]

use crate::{handle_result, wasmtime_error_t};
use std::ffi::CStr;
use std::os::raw::c_char;
use wasmtime::{Config, OptLevel, ProfilingStrategy, Strategy};

#[repr(C)]
#[derive(Clone)]
pub struct wasm_config_t {
    pub(crate) config: Config,
}

wasmtime_c_api_macros::declare_own!(wasm_config_t);

#[repr(u8)]
#[derive(Clone)]
pub enum wasmtime_strategy_t {
    WASMTIME_STRATEGY_AUTO,
    WASMTIME_STRATEGY_CRANELIFT,
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
pub extern "C" fn wasm_config_new() -> Box<wasm_config_t> {
    Box::new(wasm_config_t {
        config: Config::default(),
    })
}

#[no_mangle]
pub extern "C" fn wasmtime_config_debug_info_set(c: &mut wasm_config_t, enable: bool) {
    c.config.debug_info(enable);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_consume_fuel_set(c: &mut wasm_config_t, enable: bool) {
    c.config.consume_fuel(enable);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_epoch_interruption_set(c: &mut wasm_config_t, enable: bool) {
    c.config.epoch_interruption(enable);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_max_wasm_stack_set(c: &mut wasm_config_t, size: usize) {
    c.config.max_wasm_stack(size);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_wasm_threads_set(c: &mut wasm_config_t, enable: bool) {
    c.config.wasm_threads(enable);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_wasm_reference_types_set(c: &mut wasm_config_t, enable: bool) {
    c.config.wasm_reference_types(enable);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_wasm_simd_set(c: &mut wasm_config_t, enable: bool) {
    c.config.wasm_simd(enable);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_wasm_bulk_memory_set(c: &mut wasm_config_t, enable: bool) {
    c.config.wasm_bulk_memory(enable);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_wasm_multi_value_set(c: &mut wasm_config_t, enable: bool) {
    c.config.wasm_multi_value(enable);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_wasm_multi_memory_set(c: &mut wasm_config_t, enable: bool) {
    c.config.wasm_multi_memory(enable);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_wasm_memory64_set(c: &mut wasm_config_t, enable: bool) {
    c.config.wasm_memory64(enable);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_strategy_set(
    c: &mut wasm_config_t,
    strategy: wasmtime_strategy_t,
) {
    use wasmtime_strategy_t::*;
    c.config.strategy(match strategy {
        WASMTIME_STRATEGY_AUTO => Strategy::Auto,
        WASMTIME_STRATEGY_CRANELIFT => Strategy::Cranelift,
    });
}

#[no_mangle]
pub extern "C" fn wasmtime_config_cranelift_debug_verifier_set(
    c: &mut wasm_config_t,
    enable: bool,
) {
    c.config.cranelift_debug_verifier(enable);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_cranelift_nan_canonicalization_set(
    c: &mut wasm_config_t,
    enable: bool,
) {
    c.config.cranelift_nan_canonicalization(enable);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_cranelift_opt_level_set(
    c: &mut wasm_config_t,
    opt_level: wasmtime_opt_level_t,
) {
    use wasmtime_opt_level_t::*;
    c.config.cranelift_opt_level(match opt_level {
        WASMTIME_OPT_LEVEL_NONE => OptLevel::None,
        WASMTIME_OPT_LEVEL_SPEED => OptLevel::Speed,
        WASMTIME_OPT_LEVEL_SPEED_AND_SIZE => OptLevel::SpeedAndSize,
    });
}

#[no_mangle]
pub extern "C" fn wasmtime_config_profiler_set(
    c: &mut wasm_config_t,
    strategy: wasmtime_profiling_strategy_t,
) {
    use wasmtime_profiling_strategy_t::*;
    c.config.profiler(match strategy {
        WASMTIME_PROFILING_STRATEGY_NONE => ProfilingStrategy::None,
        WASMTIME_PROFILING_STRATEGY_JITDUMP => ProfilingStrategy::JitDump,
    });
}

#[no_mangle]
#[cfg(feature = "cache")]
pub unsafe extern "C" fn wasmtime_config_cache_config_load(
    c: &mut wasm_config_t,
    filename: *const c_char,
) -> Option<Box<wasmtime_error_t>> {
    handle_result(
        if filename.is_null() {
            c.config.cache_config_load_default()
        } else {
            match CStr::from_ptr(filename).to_str() {
                Ok(s) => c.config.cache_config_load(s),
                Err(e) => Err(e.into()),
            }
        },
        |_cfg| {},
    )
}

#[no_mangle]
pub extern "C" fn wasmtime_config_static_memory_maximum_size_set(c: &mut wasm_config_t, size: u64) {
    c.config.static_memory_maximum_size(size);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_static_memory_guard_size_set(c: &mut wasm_config_t, size: u64) {
    c.config.static_memory_guard_size(size);
}

#[no_mangle]
pub extern "C" fn wasmtime_config_dynamic_memory_guard_size_set(c: &mut wasm_config_t, size: u64) {
    c.config.dynamic_memory_guard_size(size);
}
