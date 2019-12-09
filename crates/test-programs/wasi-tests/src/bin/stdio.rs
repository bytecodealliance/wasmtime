use std::mem::MaybeUninit;
use wasi_old::wasi_unstable;
use wasi_tests::wasi_wrappers::wasi_fd_fdstat_get;

unsafe fn test_stdio() {
    for fd in &[
        wasi_unstable::STDIN_FD,
        wasi_unstable::STDOUT_FD,
        wasi_unstable::STDERR_FD,
    ] {
        let mut fdstat: wasi_unstable::FdStat = MaybeUninit::zeroed().assume_init();
        let status = wasi_fd_fdstat_get(*fd, &mut fdstat);
        assert_eq!(
            status,
            wasi_unstable::raw::__WASI_ESUCCESS,
            "fd_fdstat_get on stdio"
        );

        assert!(
            wasi_unstable::fd_renumber(*fd, *fd + 100).is_ok(),
            "renumbering stdio",
        );
    }
}

fn main() {
    // Run the tests.
    unsafe { test_stdio() }
}
