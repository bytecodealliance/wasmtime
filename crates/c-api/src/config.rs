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
pub extern "C" fn wasmtime_config_interruptable_set(c: &mut wasm_config_t, enable: bool) {
    c.config.interruptable(enable);
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
pub extern "C" fn wasmtime_config_strategy_set(
    c: &mut wasm_config_t,
    strategy: wasmtime_strategy_t,
) -> Option<Box<wasmtime_error_t>> {
    use wasmtime_strategy_t::*;
    let result = c.config.strategy(match strategy {
        WASMTIME_STRATEGY_AUTO => Strategy::Auto,
        WASMTIME_STRATEGY_CRANELIFT => Strategy::Cranelift,
        WASMTIME_STRATEGY_LIGHTBEAM => Strategy::Lightbeam,
    });
    handle_result(result, |_cfg| {})
}

#[no_mangle]
pub extern "C" fn wasmtime_config_cranelift_debug_verifier_set(
    c: &mut wasm_config_t,
    enable: bool,
) {
    c.config.cranelift_debug_verifier(enable);
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
) -> Option<Box<wasmtime_error_t>> {
    use wasmtime_profiling_strategy_t::*;
    let result = c.config.profiler(match strategy {
        WASMTIME_PROFILING_STRATEGY_NONE => ProfilingStrategy::None,
        WASMTIME_PROFILING_STRATEGY_JITDUMP => ProfilingStrategy::JitDump,
    });
    handle_result(result, |_cfg| {})
}

#[no_mangle]
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
