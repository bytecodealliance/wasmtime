use more_asserts::assert_gt;
use std::{env, mem, process};
use wasi::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_dir, close_fd, create_dir};
use wasi_tests::wasi_wrappers::{wasi_fd_fdstat_get, wasi_fd_seek, wasi_path_open};

unsafe fn test_directory_seek(dir_fd: wasi_unstable::Fd) {
    // Create a directory in the scratch directory.
    create_dir(dir_fd, "dir");

    // Open the directory and attempt to request rights for seeking.
    let mut fd: wasi_unstable::Fd = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(
        dir_fd,
        0,
        "dir",
        0,
        wasi_unstable::RIGHT_FD_SEEK,
        0,
        0,
        &mut fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a file"
    );
    assert_gt!(
        fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );

    // Attempt to seek.
    let mut newoffset = 1;
    status = wasi_fd_seek(fd, 0, wasi_unstable::WHENCE_CUR, &mut newoffset);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ENOTCAPABLE,
        "seek on a directory"
    );

    // Check if we obtained the right to seek.
    let mut fdstat: wasi_unstable::FdStat = mem::zeroed();
    status = wasi_fd_fdstat_get(fd, &mut fdstat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "calling fd_fdstat on a directory"
    );
    assert_eq!(
        fdstat.fs_filetype,
        wasi_unstable::FILETYPE_DIRECTORY,
        "expected the scratch directory to be a directory",
    );
    assert_eq!(
        (fdstat.fs_rights_base & wasi_unstable::RIGHT_FD_SEEK),
        0,
        "directory has the seek right",
    );

    // Clean up.
    close_fd(fd);
    cleanup_dir(dir_fd, "dir");
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
    unsafe { test_directory_seek(dir_fd) }
}
