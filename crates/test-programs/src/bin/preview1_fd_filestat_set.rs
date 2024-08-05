use std::{env, process, time::Duration};
use test_programs::preview1::{
    assert_errno, assert_fs_time_eq, open_scratch_directory, TestConfig,
};

unsafe fn test_fd_filestat_set_size_rw(dir_fd: wasi::Fd) {
    // Create a file in the scratch directory, opened read/write
    let file_fd = wasi::path_open(
        dir_fd,
        0,
        "file",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("failed to create file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    // Check file size
    let stat = wasi::fd_filestat_get(file_fd).expect("failed filestat");
    assert_eq!(stat.size, 0, "file size should be 0");

    // Check fd_filestat_set_size
    wasi::fd_filestat_set_size(file_fd, 100).expect("fd_filestat_set_size");

    let stat = wasi::fd_filestat_get(file_fd).expect("failed filestat 2");
    assert_eq!(stat.size, 100, "file size should be 100");

    wasi::fd_close(file_fd).expect("failed to close fd");
    wasi::path_unlink_file(dir_fd, "file").expect("failed to remove file");
}

unsafe fn test_fd_filestat_set_size_ro(dir_fd: wasi::Fd) {
    // Create a file in the scratch directory. Creating a file implies opening it for writing, so
    // we have to close and re-open read-only to observe read-only behavior.
    let file_fd = wasi::path_open(dir_fd, 0, "file", wasi::OFLAGS_CREAT, 0, 0, 0)
        .expect("failed to create file");
    wasi::fd_close(file_fd).expect("failed to close fd");

    // Open the created file read-only
    let file_fd = wasi::path_open(dir_fd, 0, "file", 0, wasi::RIGHTS_FD_READ, 0, 0)
        .expect("failed to create file");

    // Check file size
    let stat = wasi::fd_filestat_get(file_fd).expect("failed filestat");
    assert_eq!(stat.size, 0, "file size should be 0");

    // Check fd_filestat_set_size on a file opened read-only fails with EINVAL, like ftruncate is defined to do on posix
    assert_errno!(
        wasi::fd_filestat_set_size(file_fd, 100)
            .expect_err("fd_filestat_set_size should error when file is opened read-only"),
        windows => wasi::ERRNO_ACCES,
        wasi::ERRNO_INVAL
    );

    let stat = wasi::fd_filestat_get(file_fd).expect("failed filestat 2");
    assert_eq!(stat.size, 0, "file size should remain 0");

    wasi::fd_close(file_fd).expect("failed to close fd");
    wasi::path_unlink_file(dir_fd, "file").expect("failed to remove file");
}

unsafe fn test_fd_filestat_set_times(dir_fd: wasi::Fd, rights: wasi::Rights) {
    let cfg = TestConfig::from_env();

    // Create a file in the scratch directory. OFLAGS_CREAT implies opening for writing, so we will
    // close it and re-open with the desired rights (FD_READ for read only, FD_READ | FD_WRITE for
    // readwrite)
    let file_fd = wasi::path_open(dir_fd, 0, "file", wasi::OFLAGS_CREAT, 0, 0, 0)
        .expect("failed to create file");
    wasi::fd_close(file_fd).expect("failed to close fd");

    // Open the file with the rights given.
    let file_fd =
        wasi::path_open(dir_fd, 0, "file", 0, rights, 0, 0).expect("failed to create file");

    let stat = wasi::fd_filestat_get(file_fd).expect("failed filestat 2");

    // Check fd_filestat_set_times
    let old_atim = Duration::from_nanos(stat.atim);
    let new_mtim = Duration::from_nanos(stat.mtim) - cfg.fs_time_precision() * 2;
    wasi::fd_filestat_set_times(
        file_fd,
        new_mtim.as_nanos() as u64,
        new_mtim.as_nanos() as u64,
        wasi::FSTFLAGS_MTIM,
    )
    .expect("fd_filestat_set_times");

    let stat = wasi::fd_filestat_get(file_fd).expect("failed filestat 3");
    assert_eq!(stat.size, 0, "file size should remain unchanged at 0");

    // Support accuracy up to at least 1ms
    assert_fs_time_eq!(
        Duration::from_nanos(stat.mtim),
        new_mtim,
        "mtim should change"
    );
    assert_fs_time_eq!(
        Duration::from_nanos(stat.atim),
        old_atim,
        "atim should not change"
    );

    // let status = wasi_fd_filestat_set_times(file_fd, new_mtim, new_mtim, wasi::FILESTAT_SET_MTIM | wasi::FILESTAT_SET_MTIM_NOW);
    // assert_eq!(status, wasi::EINVAL, "ATIM & ATIM_NOW can't both be set");

    wasi::fd_close(file_fd).expect("failed to close fd");
    wasi::path_unlink_file(dir_fd, "file").expect("failed to remove file");
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

    unsafe { test_fd_filestat_set_size_rw(dir_fd) }
    unsafe { test_fd_filestat_set_size_ro(dir_fd) }

    // The fd_filestat_set_times function should behave the same whether the file
    // descriptor is open for read-only or read-write, because the underlying
    // permissions of the file determine whether or not the filestat can be
    // set or not, not than the open mode.
    if test_programs::preview1::config().support_dangling_filesystem() {
        // Guarding to run on non-windows filesystems. Windows rejects set-times on read-only
        // files.
        unsafe { test_fd_filestat_set_times(dir_fd, wasi::RIGHTS_FD_READ) }
    }
    unsafe { test_fd_filestat_set_times(dir_fd, wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE) }
}
