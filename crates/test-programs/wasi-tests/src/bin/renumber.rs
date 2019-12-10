use libc;
use more_asserts::assert_gt;
use std::{env, mem, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::close_fd;
use wasi_tests::wasi_wrappers::{wasi_fd_fdstat_get, wasi_path_open};

unsafe fn test_renumber(dir_fd: wasi_unstable::Fd) {
    let pre_fd: wasi_unstable::Fd = (libc::STDERR_FILENO + 1) as wasi_unstable::Fd;

    assert_gt!(dir_fd, pre_fd, "dir_fd number");

    // Create a file in the scratch directory.
    let mut fd_from = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(
        dir_fd,
        0,
        "file1",
        wasi_unstable::O_CREAT,
        wasi_unstable::RIGHT_FD_READ | wasi_unstable::RIGHT_FD_WRITE,
        0,
        0,
        &mut fd_from,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a file"
    );
    assert_gt!(
        fd_from,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );

    // Get fd_from fdstat attributes
    let mut fdstat_from: wasi_unstable::FdStat = mem::zeroed();
    status = wasi_fd_fdstat_get(fd_from, &mut fdstat_from);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "calling fd_fdstat on the open file descriptor"
    );

    // Create another file in the scratch directory.
    let mut fd_to = wasi_unstable::Fd::max_value() - 1;
    status = wasi_path_open(
        dir_fd,
        0,
        "file2",
        wasi_unstable::O_CREAT,
        wasi_unstable::RIGHT_FD_READ | wasi_unstable::RIGHT_FD_WRITE,
        0,
        0,
        &mut fd_to,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a file"
    );
    assert_gt!(
        fd_to,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );

    // Renumber fd of file1 into fd of file2
    assert!(
        wasi_unstable::fd_renumber(fd_from, fd_to).is_ok(),
        "renumbering two descriptors",
    );

    // Ensure that fd_from is closed
    assert_eq!(
        wasi_unstable::fd_close(fd_from),
        Err(wasi_unstable::EBADF),
        "closing already closed file descriptor"
    );

    // Ensure that fd_to is still open.
    let mut fdstat_to: wasi_unstable::FdStat = mem::zeroed();
    status = wasi_fd_fdstat_get(fd_to, &mut fdstat_to);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "calling fd_fdstat on the open file descriptor"
    );
    assert_eq!(
        fdstat_from.fs_filetype, fdstat_to.fs_filetype,
        "expected fd_to have the same fdstat as fd_from"
    );
    assert_eq!(
        fdstat_from.fs_flags, fdstat_to.fs_flags,
        "expected fd_to have the same fdstat as fd_from"
    );
    assert_eq!(
        fdstat_from.fs_rights_base, fdstat_to.fs_rights_base,
        "expected fd_to have the same fdstat as fd_from"
    );
    assert_eq!(
        fdstat_from.fs_rights_inheriting, fdstat_to.fs_rights_inheriting,
        "expected fd_to have the same fdstat as fd_from"
    );

    close_fd(fd_to);
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
    unsafe { test_renumber(dir_fd) }
}
