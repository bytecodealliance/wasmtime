#![allow(non_camel_case_types)]
use crate::ctx::WasiCtx;
use crate::memory::*;
use crate::wasm32;
use log::trace;

use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
pub fn proc_exit(rval: wasm32::__wasi_exitcode_t) -> () {
    trace!("proc_exit(rval={:?})", rval);
    // TODO: Rather than call std::process::exit here, we should trigger a
    // stack unwind similar to a trap.
    std::process::exit(dec_exitcode(rval) as i32);
}

#[wasi_common_cbindgen]
pub fn proc_raise(
    _wasi_ctx: &WasiCtx,
    _memory: &mut [u8],
    _sig: wasm32::__wasi_signal_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("proc_raise")
}

hostcalls! {
    pub fn args_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        argv_ptr: wasm32::uintptr_t,
        argv_buf: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn args_sizes_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        argc_ptr: wasm32::uintptr_t,
        argv_buf_size_ptr: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn environ_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        environ_ptr: wasm32::uintptr_t,
        environ_buf: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn environ_sizes_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        environ_count_ptr: wasm32::uintptr_t,
        environ_size_ptr: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn random_get(
        memory: &mut [u8],
        buf_ptr: wasm32::uintptr_t,
        buf_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn clock_res_get(
        memory: &mut [u8],
        clock_id: wasm32::__wasi_clockid_t,
        resolution_ptr: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn clock_time_get(
        memory: &mut [u8],
        clock_id: wasm32::__wasi_clockid_t,
        precision: wasm32::__wasi_timestamp_t,
        time_ptr: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn poll_oneoff(
        memory: &mut [u8],
        input: wasm32::uintptr_t,
        output: wasm32::uintptr_t,
        nsubscriptions: wasm32::size_t,
        nevents: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn sched_yield() -> wasm32::__wasi_errno_t;
}
