use more_asserts::assert_gt;
use std::{env, process};
use wasi_tests::{assert_errno, open_scratch_directory};

unsafe fn test_directory_seek(dir_fd: wasi::Fd) {
    // Create a directory in the scratch directory.
    wasi::path_create_directory(dir_fd, "dir").expect("failed to make directory");

    // Open the directory and attempt to request rights for seeking.
    let fd = wasi::path_open(
        dir_fd,
        0,
        "dir",
        wasi::OFLAGS_DIRECTORY,
        wasi::RIGHTS_FD_SEEK,
        0,
        0,
    )
    .expect("failed to open file");
    assert_gt!(
        fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    // Attempt to seek.
    assert_errno!(
        wasi::fd_seek(fd, 0, wasi::WHENCE_CUR)
            .expect_err("seek on a directory")
            .raw_error(),
        wasi::ERRNO_BADF
    );

    // Check if we obtained the right to seek.
    let fdstat = wasi::fd_fdstat_get(fd).expect("failed to fdstat");
    assert_eq!(
        fdstat.fs_filetype,
        wasi::FILETYPE_DIRECTORY,
        "expected the scratch directory to be a directory",
    );
    assert_eq!(
        (fdstat.fs_rights_base & wasi::RIGHTS_FD_SEEK),
        0,
        "directory does NOT have the seek right",
    );

    // Clean up.
    wasi::fd_close(fd).expect("failed to close fd");
    wasi::path_remove_directory(dir_fd, "dir").expect("failed to remove dir");
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
