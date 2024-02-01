use crate::{wasm_byte_vec_t, wasm_name_t, wasmtime_error_t, wasmtime_module_t, wasmtime_store_t};
use std::mem::MaybeUninit;
use std::str::from_utf8;
use std::time::Duration;
use wasmtime::GuestProfiler;

pub struct wasm_guestprofiler_t {
    guest_profiler: GuestProfiler,
}

wasmtime_c_api_macros::declare_own!(wasm_guestprofiler_t);

#[no_mangle]
pub unsafe extern "C" fn wasmtime_guestprofiler_new(
    module_name: &wasm_name_t,
    interval_nanos: u64,
    modules_size: usize,
    modules_name: *const &wasm_name_t,
    modules_module: *const &wasmtime_module_t,
) -> Box<wasm_guestprofiler_t> {
    let modules_size = modules_size.try_into().unwrap();
    let module_name = from_utf8(&module_name.as_slice()).expect("not valid utf-8");
    let list = (0..modules_size)
        .map(|i| {
            (
                from_utf8(modules_name.offset(i).as_ref().unwrap().as_slice())
                    .expect("not valid utf-8")
                    .to_owned(),
                modules_module.offset(i).as_ref().unwrap().module.clone(),
            )
        })
        .collect();
    Box::new(wasm_guestprofiler_t {
        guest_profiler: GuestProfiler::new(module_name, Duration::from_nanos(interval_nanos), list),
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_guestprofiler_sample(
    guestprofiler: &mut wasm_guestprofiler_t,
    store: &mut wasmtime_store_t,
) {
    guestprofiler.guest_profiler.sample(&store.store);
}

#[no_mangle]
pub unsafe extern "C" fn wasmtime_guestprofiler_finish(
    guestprofiler: &mut MaybeUninit<wasm_guestprofiler_t>,
    out: &mut wasm_byte_vec_t,
) -> Option<Box<wasmtime_error_t>> {
    let mut buf = vec![];
    match guestprofiler
        .assume_init_read()
        .guest_profiler
        .finish(&mut buf)
    {
        Ok(()) => {
            out.set_buffer(buf);
            None
        }
        Err(e) => Some(Box::new(e.into())),
    }
}
