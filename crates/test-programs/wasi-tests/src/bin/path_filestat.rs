use more_asserts::assert_gt;
use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn test_path_filestat(dir_fd: wasi::Fd) {
    let mut fdstat = wasi::fd_fdstat_get(dir_fd).expect("fd_fdstat_get");
    assert_ne!(
        fdstat.fs_rights_base & wasi::RIGHTS_PATH_FILESTAT_GET,
        0,
        "the scratch directory should have RIGHT_PATH_FILESTAT_GET as base right",
    );
    assert_ne!(
        fdstat.fs_rights_inheriting & wasi::RIGHTS_PATH_FILESTAT_GET,
        0,
        "the scratch directory should have RIGHT_PATH_FILESTAT_GET as base right",
    );

    // Create a file in the scratch directory.
    let file_fd = wasi::path_open(
        dir_fd,
        0,
        "file",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE | wasi::RIGHTS_PATH_FILESTAT_GET,
        0,
        // Pass some flags for later retrieval
        wasi::FDFLAGS_APPEND | wasi::FDFLAGS_SYNC,
    )
    .expect("opening a file");
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    fdstat = wasi::fd_fdstat_get(file_fd).expect("fd_fdstat_get");
    assert_eq!(
        fdstat.fs_rights_base & wasi::RIGHTS_PATH_FILESTAT_GET,
        0,
        "files shouldn't have rights for path_* syscalls even if manually given",
    );
    assert_eq!(
        fdstat.fs_rights_inheriting & wasi::RIGHTS_PATH_FILESTAT_GET,
        0,
        "files shouldn't have rights for path_* syscalls even if manually given",
    );
    assert_ne!(
        fdstat.fs_flags & (wasi::FDFLAGS_APPEND | wasi::FDFLAGS_SYNC),
        0,
        "file should have the same flags used to create the file"
    );

    // Check file size
    let mut stat = wasi::path_filestat_get(dir_fd, 0, "file").expect("reading file stats");
    assert_eq!(stat.size, 0, "file size should be 0");

    // Check path_filestat_set_times
    let old_atim = stat.atim;
    let new_mtim = stat.mtim - 100;
    wasi::path_filestat_set_times(
        dir_fd,
        0,
        "file",
        // on purpose: the syscall should not touch atim, because
        // neither of the ATIM flags is set
        new_mtim,
        new_mtim,
        wasi::FSTFLAGS_MTIM,
    )
    .expect("path_filestat_set_times should succeed");

    stat = wasi::path_filestat_get(dir_fd, 0, "file")
        .expect("reading file stats after path_filestat_set_times");
    assert_eq!(stat.mtim, new_mtim, "mtim should change");
    assert_eq!(stat.atim, old_atim, "atim should not change");

    assert_eq!(
        wasi::path_filestat_set_times(
            dir_fd,
            0,
            "file",
            new_mtim,
            new_mtim,
            wasi::FSTFLAGS_MTIM | wasi::FSTFLAGS_MTIM_NOW,
        )
        .expect_err("MTIM and MTIM_NOW can't both be set")
        .raw_error(),
        wasi::ERRNO_INVAL,
        "errno should be ERRNO_INVAL"
    );

    // check if the times were untouched
    stat = wasi::path_filestat_get(dir_fd, 0, "file")
        .expect("reading file stats after ERRNO_INVAL fd_filestat_set_times");
    assert_eq!(stat.mtim, new_mtim, "mtim should not change");
    assert_eq!(stat.atim, old_atim, "atim should not change");

    let new_atim = old_atim - 100;
    assert_eq!(
        wasi::path_filestat_set_times(
            dir_fd,
            0,
            "file",
            new_atim,
            new_atim,
            wasi::FSTFLAGS_ATIM | wasi::FSTFLAGS_ATIM_NOW,
        )
        .expect_err("ATIM & ATIM_NOW can't both be set")
        .raw_error(),
        wasi::ERRNO_INVAL,
        "errno should be ERRNO_INVAL"
    );

    // check if the times were untouched
    stat = wasi::path_filestat_get(dir_fd, 0, "file")
        .expect("reading file stats after ERRNO_INVAL path_filestat_set_times");
    assert_eq!(stat.mtim, new_mtim, "mtim should not change");
    assert_eq!(stat.atim, old_atim, "atim should not change");

    wasi::fd_close(file_fd).expect("closing a file");
    wasi::path_unlink_file(dir_fd, "file").expect("removing a file");
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
    unsafe { test_path_filestat(dir_fd) }
}
