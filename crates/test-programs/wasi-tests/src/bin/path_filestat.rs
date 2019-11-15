use more_asserts::assert_gt;
use std::{env, process};
use wasi::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_file, close_fd};
use wasi_tests::wasi_wrappers::{
    wasi_fd_fdstat_get, wasi_path_filestat_get, wasi_path_filestat_set_times, wasi_path_open,
};

unsafe fn test_path_filestat(dir_fd: wasi_unstable::Fd) {
    let mut fdstat: wasi_unstable::FdStat = std::mem::zeroed();
    let status = wasi_fd_fdstat_get(dir_fd, &mut fdstat);
    assert_eq!(status, wasi_unstable::raw::__WASI_ESUCCESS, "fd_fdstat_get");

    assert_ne!(
        fdstat.fs_rights_base & wasi_unstable::RIGHT_PATH_FILESTAT_GET,
        0,
        "the scratch directory should have RIGHT_PATH_FILESTAT_GET as base right",
    );
    assert_ne!(
        fdstat.fs_rights_inheriting & wasi_unstable::RIGHT_PATH_FILESTAT_GET,
        0,
        "the scratch directory should have RIGHT_PATH_FILESTAT_GET as base right",
    );

    // Create a file in the scratch directory.
    let mut file_fd = wasi_unstable::Fd::max_value() - 1;
    let filename = "file";
    let status = wasi_path_open(
        dir_fd,
        0,
        filename,
        wasi_unstable::O_CREAT,
        wasi_unstable::RIGHT_FD_READ
            | wasi_unstable::RIGHT_FD_WRITE
            | wasi_unstable::RIGHT_PATH_FILESTAT_GET,
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

    let status = wasi_fd_fdstat_get(file_fd, &mut fdstat);
    assert_eq!(status, wasi_unstable::raw::__WASI_ESUCCESS, "fd_fdstat_get");

    assert_eq!(
        fdstat.fs_rights_base & wasi_unstable::RIGHT_PATH_FILESTAT_GET,
        0,
        "files shouldn't have rights for path_* syscalls even if manually given",
    );
    assert_eq!(
        fdstat.fs_rights_inheriting & wasi_unstable::RIGHT_PATH_FILESTAT_GET,
        0,
        "files shouldn't have rights for path_* syscalls even if manually given",
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
    let status = wasi_path_filestat_get(dir_fd, 0, filename, filename.len(), &mut stat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading file stats"
    );
    assert_eq!(stat.st_size, 0, "file size should be 0");

    // Check path_filestat_set_times
    let old_atim = stat.st_atim;
    let new_mtim = stat.st_mtim - 100;
    assert!(
        wasi_path_filestat_set_times(
            dir_fd,
            0,
            filename,
            // on purpose: the syscall should not touch atim, because
            // neither of the ATIM flags is set
            new_mtim,
            new_mtim,
            wasi_unstable::FILESTAT_SET_MTIM,
        )
        .is_ok(),
        "path_filestat_set_times should succeed"
    );

    let status = wasi_path_filestat_get(dir_fd, 0, filename, filename.len(), &mut stat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading file stats after path_filestat_set_times"
    );
    assert_eq!(stat.st_mtim, new_mtim, "mtim should change");
    assert_eq!(stat.st_atim, old_atim, "atim should not change");

    assert_eq!(
        wasi_path_filestat_set_times(
            dir_fd,
            0,
            filename,
            new_mtim,
            new_mtim,
            wasi_unstable::FILESTAT_SET_MTIM | wasi_unstable::FILESTAT_SET_MTIM_NOW,
        ),
        Err(wasi_unstable::EINVAL),
        "MTIM & MTIM_NOW can't both be set"
    );

    // check if the times were untouched
    let status = wasi_path_filestat_get(dir_fd, 0, filename, filename.len(), &mut stat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading file stats after EINVAL fd_filestat_set_times"
    );
    assert_eq!(stat.st_mtim, new_mtim, "mtim should not change");
    assert_eq!(stat.st_atim, old_atim, "atim should not change");

    let new_atim = old_atim - 100;
    assert_eq!(
        wasi_path_filestat_set_times(
            dir_fd,
            0,
            filename,
            new_atim,
            new_atim,
            wasi_unstable::FILESTAT_SET_ATIM | wasi_unstable::FILESTAT_SET_ATIM_NOW,
        ),
        Err(wasi_unstable::EINVAL),
        "ATIM & ATIM_NOW can't both be set"
    );

    // check if the times were untouched
    let status = wasi_path_filestat_get(dir_fd, 0, filename, filename.len(), &mut stat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading file stats after EINVAL path_filestat_set_times"
    );
    assert_eq!(stat.st_mtim, new_mtim, "mtim should not change");
    assert_eq!(stat.st_atim, old_atim, "atim should not change");

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
    unsafe { test_path_filestat(dir_fd) }
}
