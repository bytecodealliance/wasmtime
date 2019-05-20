#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
use super::host_impl;

use crate::memory::*;
use crate::{host, wasm32};

use std::cmp;
use std::time::SystemTime;
use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
pub fn clock_res_get(
    memory: &mut [u8],
    clock_id: wasm32::__wasi_clockid_t,
    resolution_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("clock_res_get")
}

#[wasi_common_cbindgen]
pub fn clock_time_get(
    memory: &mut [u8],
    clock_id: wasm32::__wasi_clockid_t,
    precision: wasm32::__wasi_timestamp_t,
    time_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("clock_time_get")
}

#[wasi_common_cbindgen]
pub fn poll_oneoff(
    memory: &mut [u8],
    input: wasm32::uintptr_t,
    output: wasm32::uintptr_t,
    nsubscriptions: wasm32::size_t,
    nevents: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("poll_oneoff")
}

#[wasi_common_cbindgen]
pub fn sched_yield() -> wasm32::__wasi_errno_t {
    unimplemented!("sched_yield")
}
