#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

use std::{env, process};
use test_programs::preview1::{assert_errno, open_scratch_directory};

unsafe fn test_close_preopen(dir_fd: wasip1::Fd) {
    let pre_fd: wasip1::Fd = (libc::STDERR_FILENO + 1) as wasip1::Fd;

    assert!(dir_fd > pre_fd, "dir_fd number");

    // Try to close a preopened directory handle.
    wasip1::fd_close(pre_fd).expect("closing a preopened file descriptor");

    // Ensure that dir_fd is still open.
    let dir_fdstat = wasip1::fd_fdstat_get(dir_fd).expect("failed fd_fdstat_get");
    assert_eq!(
        dir_fdstat.fs_filetype,
        wasip1::FILETYPE_DIRECTORY,
        "expected the scratch directory to be a directory",
    );

    // Ensure that pre_fd is closed.
    assert_errno!(
        wasip1::fd_fdstat_get(pre_fd).expect_err("failed fd_fdstat_get"),
        wasip1::ERRNO_BADF
    );
}

fn main() {
    let mut args = env::args();
    let prog = args.next().unwrap();
    let arg = if let Some(arg) = args.next() {
        arg
    } else {
        eprintln!("usage: {prog} <scratch directory>");
        process::exit(1);
    };

    // Open scratch directory
    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe { test_close_preopen(dir_fd) }
}
