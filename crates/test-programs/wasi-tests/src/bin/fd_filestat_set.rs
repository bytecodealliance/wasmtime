use libc;
use more_asserts::assert_gt;
use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_file, close_fd};
use wasi_tests::wasi_wrappers::{wasi_fd_filestat_get, wasi_path_open};

unsafe fn test_fd_filestat_set(dir_fd: wasi_unstable::Fd) {
    // Create a file in the scratch directory.
    let mut file_fd = wasi_unstable::Fd::max_value() - 1;
    let status = wasi_path_open(
        dir_fd,
        0,
        "file",
        wasi_unstable::O_CREAT,
        wasi_unstable::RIGHT_FD_READ | wasi_unstable::RIGHT_FD_WRITE,
        0,
        0,
        &mut file_fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a file"
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );

    // Check file size
    let mut stat = wasi_unstable::FileStat {
        st_dev: 0,
        st_ino: 0,
        st_filetype: 0,
        st_nlink: 0,
        st_size: 0,
        st_atim: 0,
        st_mtim: 0,
        st_ctim: 0,
    };
    let status = wasi_fd_filestat_get(file_fd, &mut stat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading file stats"
    );
    assert_eq!(stat.st_size, 0, "file size should be 0");

    // Check fd_filestat_set_size
    assert!(
        wasi_unstable::fd_filestat_set_size(file_fd, 100).is_ok(),
        "fd_filestat_set_size"
    );

    let status = wasi_fd_filestat_get(file_fd, &mut stat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading file stats after fd_filestat_set_size"
    );
    assert_eq!(stat.st_size, 100, "file size should be 100");

    // Check fd_filestat_set_times
    let old_atim = stat.st_atim;
    let new_mtim = stat.st_mtim - 100;
    assert!(
        wasi_unstable::fd_filestat_set_times(
            file_fd,
            new_mtim,
            new_mtim,
            wasi_unstable::FILESTAT_SET_MTIM,
        )
        .is_ok(),
        "fd_filestat_set_times"
    );

    let status = wasi_fd_filestat_get(file_fd, &mut stat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading file stats after fd_filestat_set_times"
    );
    assert_eq!(
        stat.st_size, 100,
        "file size should remain unchanged at 100"
    );
    assert_eq!(stat.st_mtim, new_mtim, "mtim should change");
    assert_eq!(stat.st_atim, old_atim, "atim should not change");

    // let status = wasi_fd_filestat_set_times(file_fd, new_mtim, new_mtim, wasi_unstable::FILESTAT_SET_MTIM | wasi_unstable::FILESTAT_SET_MTIM_NOW);
    // assert_eq!(status, wasi_unstable::EINVAL, "ATIM & ATIM_NOW can't both be set");

    close_fd(file_fd);
    cleanup_file(dir_fd, "file");
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
    unsafe { test_fd_filestat_set(dir_fd) }
}
