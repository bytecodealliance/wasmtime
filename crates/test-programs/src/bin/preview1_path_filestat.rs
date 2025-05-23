#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

use std::{env, process, time::Duration};
use test_programs::preview1::{
    TestConfig, assert_errno, assert_fs_time_eq, open_scratch_directory,
};

unsafe fn test_path_filestat(dir_fd: wasip1::Fd) {
    let cfg = TestConfig::from_env();
    let fdflags = wasip1::FDFLAGS_APPEND;

    // Create a file in the scratch directory.
    let file_fd = wasip1::path_open(
        dir_fd,
        0,
        "file",
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        // Pass some flags for later retrieval
        fdflags,
    )
    .expect("opening a file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    let fdstat = wasip1::fd_fdstat_get(file_fd).expect("fd_fdstat_get");
    assert_eq!(
        fdstat.fs_flags & wasip1::FDFLAGS_APPEND,
        wasip1::FDFLAGS_APPEND,
        "file should have the APPEND fdflag used to create the file"
    );
    assert_errno!(
        wasip1::path_open(
            dir_fd,
            0,
            "file",
            0,
            wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
            0,
            wasip1::FDFLAGS_SYNC,
        )
        .expect_err("FDFLAGS_SYNC not supported by platform"),
        wasip1::ERRNO_NOTSUP
    );

    // Check file size
    let file_stat = wasip1::path_filestat_get(dir_fd, 0, "file").expect("reading file stats");
    assert_eq!(file_stat.size, 0, "file size should be 0");

    // Check path_filestat_set_times
    let new_mtim = Duration::from_nanos(file_stat.mtim) - 2 * cfg.fs_time_precision();
    wasip1::path_filestat_set_times(
        dir_fd,
        0,
        "file",
        0,
        new_mtim.as_nanos() as u64,
        wasip1::FSTFLAGS_MTIM,
    )
    .expect("path_filestat_set_times should succeed");

    let modified_file_stat = wasip1::path_filestat_get(dir_fd, 0, "file")
        .expect("reading file stats after path_filestat_set_times");

    assert_fs_time_eq!(
        Duration::from_nanos(modified_file_stat.mtim),
        new_mtim,
        "mtim should change"
    );

    assert_errno!(
        wasip1::path_filestat_set_times(
            dir_fd,
            0,
            "file",
            0,
            new_mtim.as_nanos() as u64,
            wasip1::FSTFLAGS_MTIM | wasip1::FSTFLAGS_MTIM_NOW,
        )
        .expect_err("MTIM and MTIM_NOW can't both be set"),
        wasip1::ERRNO_INVAL
    );

    // check if the times were untouched
    let unmodified_file_stat = wasip1::path_filestat_get(dir_fd, 0, "file")
        .expect("reading file stats after ERRNO_INVAL fd_filestat_set_times");

    assert_fs_time_eq!(
        Duration::from_nanos(unmodified_file_stat.mtim),
        new_mtim,
        "mtim should not change"
    );

    // Invalid arguments to set_times:
    assert_errno!(
        wasip1::path_filestat_set_times(
            dir_fd,
            0,
            "file",
            0,
            0,
            wasip1::FSTFLAGS_ATIM | wasip1::FSTFLAGS_ATIM_NOW,
        )
        .expect_err("ATIM & ATIM_NOW can't both be set"),
        wasip1::ERRNO_INVAL
    );

    wasip1::fd_close(file_fd).expect("closing a file");
    wasip1::path_unlink_file(dir_fd, "file").expect("removing a file");
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
    unsafe { test_path_filestat(dir_fd) }
}
