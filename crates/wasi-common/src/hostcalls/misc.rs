#![allow(non_camel_case_types)]
use crate::ctx::WasiCtx;
use crate::{hostcalls_impl, wasi, wasi32};
use log::trace;
use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
pub unsafe fn proc_exit(_wasi_ctx: &WasiCtx, _memory: &mut [u8], rval: wasi::__wasi_exitcode_t) {
    trace!("proc_exit(rval={:?})", rval);
    // TODO: Rather than call std::process::exit here, we should trigger a
    // stack unwind similar to a trap.
    std::process::exit(rval as i32);
}

#[wasi_common_cbindgen]
pub unsafe fn proc_raise(
    _wasi_ctx: &WasiCtx,
    _memory: &mut [u8],
    _sig: wasi::__wasi_signal_t,
) -> wasi::__wasi_errno_t {
    unimplemented!("proc_raise")
}

hostcalls! {
    pub unsafe fn args_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        argv_ptr: wasi32::uintptr_t,
        argv_buf: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn args_sizes_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        argc_ptr: wasi32::uintptr_t,
        argv_buf_size_ptr: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn environ_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        environ_ptr: wasi32::uintptr_t,
        environ_buf: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn environ_sizes_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        environ_count_ptr: wasi32::uintptr_t,
        environ_size_ptr: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn random_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        buf_ptr: wasi32::uintptr_t,
        buf_len: wasi32::size_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn clock_res_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        clock_id: wasi::__wasi_clockid_t,
        resolution_ptr: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn clock_time_get(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        clock_id: wasi::__wasi_clockid_t,
        precision: wasi::__wasi_timestamp_t,
        time_ptr: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn poll_oneoff(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        input: wasi32::uintptr_t,
        output: wasi32::uintptr_t,
        nsubscriptions: wasi32::size_t,
        nevents: wasi32::uintptr_t,
    ) -> wasi::__wasi_errno_t;

    pub unsafe fn sched_yield(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
    ) -> wasi::__wasi_errno_t;
}
