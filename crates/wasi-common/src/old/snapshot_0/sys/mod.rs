use crate::old::snapshot_0::wasi;
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(unix)] {
        mod unix;
        pub(crate) use self::unix::*;

        pub(crate) fn errno_from_host(err: i32) -> wasi::__wasi_errno_t {
            host_impl::errno_from_nix(nix::errno::from_i32(err)).as_wasi_errno()
        }
    } else if #[cfg(windows)] {
        mod windows;
        pub(crate) use self::windows::*;

        pub(crate) fn errno_from_host(err: i32) -> wasi::__wasi_errno_t {
            host_impl::errno_from_win(winx::winerror::WinError::from_u32(err as u32))
        }
    } else {
        compile_error!("wasi-common doesn't compile for this platform yet");
    }
}
