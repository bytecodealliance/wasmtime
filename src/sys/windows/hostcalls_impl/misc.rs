#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
use super::host_impl;
use crate::memory::*;
use crate::{host, wasm32};

use wasi_common_cbindgen::wasi_common_cbindgen;

pub(crate) fn clock_res_get(
    clock_id: host::__wasi_clockid_t,
) -> Result<host::__wasi_timestamp_t, host::__wasi_errno_t> {
    unimplemented!("clock_res_get")
}

pub(crate) fn clock_time_get(
    clock_id: host::__wasi_clockid_t,
) -> Result<host::__wasi_timestamp_t, host::__wasi_errno_t> {
    unimplemented!("clock_time_get")
}

pub(crate) fn poll_oneoff(
    input: Vec<Result<host::__wasi_subscription_t, host::__wasi_errno_t>>,
    output_slice: &mut [wasm32::__wasi_event_t],
) -> Result<wasm32::size_t, host::__wasi_errno_t> {
    unimplemented!("poll_oneoff")
}

pub(crate) fn sched_yield() -> Result<(), host::__wasi_errno_t> {
    unimplemented!("sched_yield")
}
