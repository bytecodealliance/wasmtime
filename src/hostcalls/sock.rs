#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
use crate::ctx::WasiCtx;
use crate::wasm32;
use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
pub unsafe fn sock_recv(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    sock: wasm32::__wasi_fd_t,
    ri_data: wasm32::uintptr_t,
    ri_data_len: wasm32::size_t,
    ri_flags: wasm32::__wasi_riflags_t,
    ro_datalen: wasm32::uintptr_t,
    ro_flags: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("sock_recv")
}

#[wasi_common_cbindgen]
pub unsafe fn sock_send(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    sock: wasm32::__wasi_fd_t,
    si_data: wasm32::uintptr_t,
    si_data_len: wasm32::size_t,
    si_flags: wasm32::__wasi_siflags_t,
    so_datalen: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("sock_send")
}

#[wasi_common_cbindgen]
pub unsafe fn sock_shutdown(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    sock: wasm32::__wasi_fd_t,
    how: wasm32::__wasi_sdflags_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("sock_shutdown")
}
