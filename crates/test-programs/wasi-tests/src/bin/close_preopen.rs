use more_asserts::assert_gt;
use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn test_close_preopen(dir_fd: wasi::Fd) {
    let pre_fd: wasi::Fd = (libc::STDERR_FILENO + 1) as wasi::Fd;

    assert_gt!(dir_fd, pre_fd, "dir_fd number");

    // Try to close a preopened directory handle.
    assert_eq!(
        wasi::fd_close(pre_fd)
            .expect_err("closing a preopened file descriptor")
            .raw_error(),
        wasi::ERRNO_NOTSUP,
        "errno should ERRNO_NOTSUP",
    );

    // Try to renumber over a preopened directory handle.
    assert_eq!(
        wasi::fd_renumber(dir_fd, pre_fd)
            .expect_err("renumbering over a preopened file descriptor")
            .raw_error(),
        wasi::ERRNO_NOTSUP,
        "errno should be ERRNO_NOTSUP",
    );

    // Ensure that dir_fd is still open.
    let dir_fdstat = wasi::fd_fdstat_get(dir_fd).expect("failed fd_fdstat_get");
    assert_eq!(
        dir_fdstat.fs_filetype,
        wasi::FILETYPE_DIRECTORY,
        "expected the scratch directory to be a directory",
    );

    // Try to renumber a preopened directory handle.
    assert_eq!(
        wasi::fd_renumber(pre_fd, dir_fd)
            .expect_err("renumbering over a preopened file descriptor")
            .raw_error(),
        wasi::ERRNO_NOTSUP,
        "errno should be ERRNO_NOTSUP",
    );

    // Ensure that dir_fd is still open.
    let dir_fdstat = wasi::fd_fdstat_get(dir_fd).expect("failed fd_fdstat_get");
    assert_eq!(
        dir_fdstat.fs_filetype,
        wasi::FILETYPE_DIRECTORY,
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
