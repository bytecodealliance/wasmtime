use crate::{
    handle_result, wasm_byte_vec_t, wasm_engine_t, wasm_name_t, wasmtime_error_t,
    wasmtime_module_t, wasmtime_store_t,
};
use std::str::from_utf8;
use std::time::Duration;
use wasmtime::{GuestProfiler, GuestProfilerModules, Module};

pub struct wasmtime_guestprofiler_t {
    guest_profiler: GuestProfiler,
}

wasmtime_c_api_macros::declare_own!(wasmtime_guestprofiler_t);

pub struct wasmtime_guestprofiler_module_set_t {
    modules: GuestProfilerModules,
}

wasmtime_c_api_macros::declare_own!(wasmtime_guestprofiler_module_set_t);

#[repr(C)]
pub struct wasmtime_guestprofiler_modules_t<'a> {
    name: &'a wasm_name_t,
    module: &'a wasmtime_module_t,
}

/// Converts a C array of name/module pairs into the form the Rust API takes.
unsafe fn module_list(
    modules: *const wasmtime_guestprofiler_modules_t,
    modules_len: usize,
) -> Vec<(String, Module)> {
    crate::slice_from_raw_parts(modules, modules_len)
        .iter()
        .map(|entry| {
            (
                from_utf8(entry.name.as_slice())
                    .expect("not valid utf-8")
                    .to_owned(),
                entry.module.module.clone(),
            )
        })
        .collect()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_guestprofiler_new(
    engine: &wasm_engine_t,
    module_name: &wasm_name_t,
    interval_nanos: u64,
    modules: *const wasmtime_guestprofiler_modules_t,
    modules_len: usize,
) -> Box<wasmtime_guestprofiler_t> {
    let module_name = from_utf8(&module_name.as_slice()).expect("not valid utf-8");
    let list = module_list(modules, modules_len);
    Box::new(wasmtime_guestprofiler_t {
        guest_profiler: GuestProfiler::new(
            &engine.engine,
            module_name,
            Duration::from_nanos(interval_nanos),
            list,
        )
        .unwrap(),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_guestprofiler_module_set_new(
    engine: &wasm_engine_t,
    modules: *const wasmtime_guestprofiler_modules_t,
    modules_len: usize,
    out: &mut *mut wasmtime_guestprofiler_module_set_t,
) -> Option<Box<wasmtime_error_t>> {
    let list = module_list(modules, modules_len);
    handle_result(GuestProfilerModules::new(&engine.engine, list), |modules| {
        *out = Box::into_raw(Box::new(wasmtime_guestprofiler_module_set_t { modules }));
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_guestprofiler_module_set_clone(
    set: &wasmtime_guestprofiler_module_set_t,
) -> Box<wasmtime_guestprofiler_module_set_t> {
    Box::new(wasmtime_guestprofiler_module_set_t {
        modules: set.modules.clone(),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_guestprofiler_new_with_module_set(
    module_name: &wasm_name_t,
    interval_nanos: u64,
    modules: &wasmtime_guestprofiler_module_set_t,
) -> Box<wasmtime_guestprofiler_t> {
    let module_name = from_utf8(&module_name.as_slice()).expect("not valid utf-8");
    Box::new(wasmtime_guestprofiler_t {
        guest_profiler: GuestProfiler::new_with_modules(
            module_name,
            Duration::from_nanos(interval_nanos),
            &modules.modules,
        ),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_guestprofiler_sample(
    guestprofiler: &mut wasmtime_guestprofiler_t,
    store: &wasmtime_store_t,
    delta_nanos: u64,
) {
    guestprofiler
        .guest_profiler
        .sample(&store.store, Duration::from_nanos(delta_nanos));
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_guestprofiler_finish(
    guestprofiler: Box<wasmtime_guestprofiler_t>,
    out: &mut wasm_byte_vec_t,
) -> Option<Box<wasmtime_error_t>> {
    let mut buf = vec![];
    match guestprofiler.guest_profiler.finish(&mut buf) {
        Ok(()) => {
            out.set_buffer(buf);
            None
        }
        Err(e) => Some(Box::new(e.into())),
    }
}
