#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
use crate::old::snapshot_0::ctx::WasiCtx;
use crate::old::snapshot_0::{wasi, wasi32};
use wasi_common_cbindgen::wasi_common_cbindgen_old;

#[wasi_common_cbindgen_old]
pub unsafe fn sock_recv(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    sock: wasi::__wasi_fd_t,
    ri_data: wasi32::uintptr_t,
    ri_data_len: wasi32::size_t,
    ri_flags: wasi::__wasi_riflags_t,
    ro_datalen: wasi32::uintptr_t,
    ro_flags: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    unimplemented!("sock_recv")
}

#[wasi_common_cbindgen_old]
pub unsafe fn sock_send(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    sock: wasi::__wasi_fd_t,
    si_data: wasi32::uintptr_t,
    si_data_len: wasi32::size_t,
    si_flags: wasi::__wasi_siflags_t,
    so_datalen: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    unimplemented!("sock_send")
}

#[wasi_common_cbindgen_old]
pub unsafe fn sock_shutdown(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    sock: wasi::__wasi_fd_t,
    how: wasi::__wasi_sdflags_t,
) -> wasi::__wasi_errno_t {
    unimplemented!("sock_shutdown")
}
