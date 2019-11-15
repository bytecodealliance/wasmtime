use libc;
use more_asserts::assert_gt;
use std::{env, mem, process};
use wasi::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::wasi_wrappers::wasi_fd_fdstat_get;

unsafe fn test_close_preopen(dir_fd: wasi_unstable::Fd) {
    let pre_fd: wasi_unstable::Fd = (libc::STDERR_FILENO + 1) as wasi_unstable::Fd;

    assert_gt!(dir_fd, pre_fd, "dir_fd number");

    // Try to close a preopened directory handle.
    assert_eq!(
        wasi_unstable::fd_close(pre_fd),
        Err(wasi_unstable::ENOTSUP),
        "closing a preopened file descriptor",
    );

    // Try to renumber over a preopened directory handle.
    assert_eq!(
        wasi_unstable::fd_renumber(dir_fd, pre_fd),
        Err(wasi_unstable::ENOTSUP),
        "renumbering over a preopened file descriptor",
    );

    // Ensure that dir_fd is still open.
    let mut dir_fdstat: wasi_unstable::FdStat = mem::zeroed();
    let mut status = wasi_fd_fdstat_get(dir_fd, &mut dir_fdstat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "calling fd_fdstat on the scratch directory"
    );
    assert_eq!(
        dir_fdstat.fs_filetype,
        wasi_unstable::FILETYPE_DIRECTORY,
        "expected the scratch directory to be a directory",
    );

    // Try to renumber a preopened directory handle.
    assert_eq!(
        wasi_unstable::fd_renumber(pre_fd, dir_fd),
        Err(wasi_unstable::ENOTSUP),
        "renumbering over a preopened file descriptor",
    );

    // Ensure that dir_fd is still open.
    status = wasi_fd_fdstat_get(dir_fd, &mut dir_fdstat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "calling fd_fdstat on the scratch directory"
    );
    assert_eq!(
        dir_fdstat.fs_filetype,
        wasi_unstable::FILETYPE_DIRECTORY,
        "expected the scratch directory to be a directory",
    );
}

fn main() {
    let mut args = env::args();
    let prog = args.next().unwrap();
    let arg = if let Some(arg) = args.next() {
        arg
    } else {
        eprintln!("usage: {} <scratch directory>", prog);
        process::exit(1);
    };

    // Open scratch directory
    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe { test_close_preopen(dir_fd) }
}
