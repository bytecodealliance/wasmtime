use crate::ctx::WasiCtx;
use crate::wasi::{self, WasiResult};
use crate::wasi32;

pub fn sock_recv(
    _wasi_ctx: &WasiCtx,
    _memory: &mut [u8],
    _sock: wasi::__wasi_fd_t,
    _ri_data: wasi32::uintptr_t,
    _ri_data_len: wasi32::size_t,
    _ri_flags: wasi::__wasi_riflags_t,
    _ro_datalen: wasi32::uintptr_t,
    _ro_flags: wasi32::uintptr_t,
) -> WasiResult<()> {
    unimplemented!("sock_recv")
}

pub fn sock_send(
    _wasi_ctx: &WasiCtx,
    _memory: &mut [u8],
    _sock: wasi::__wasi_fd_t,
    _si_data: wasi32::uintptr_t,
    _si_data_len: wasi32::size_t,
    _si_flags: wasi::__wasi_siflags_t,
    _so_datalen: wasi32::uintptr_t,
) -> WasiResult<()> {
    unimplemented!("sock_send")
}

pub fn sock_shutdown(
    _wasi_ctx: &WasiCtx,
    _memory: &mut [u8],
    _sock: wasi::__wasi_fd_t,
    _how: wasi::__wasi_sdflags_t,
) -> WasiResult<()> {
    unimplemented!("sock_shutdown")
}
