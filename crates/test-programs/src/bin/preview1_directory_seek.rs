use std::{env, process};
use test_programs::preview1::{assert_errno, open_scratch_directory};

unsafe fn test_directory_seek(dir_fd: wasip1::Fd) {
    // Create a directory in the scratch directory.
    wasip1::path_create_directory(dir_fd, "dir").expect("failed to make directory");

    // Open the directory and attempt to request rights for seeking.
    let fd = wasip1::path_open(dir_fd, 0, "dir", wasip1::OFLAGS_DIRECTORY, 0, 0, 0)
        .expect("failed to open file");
    assert!(
        fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    // Attempt to seek.
    assert_errno!(
        wasip1::fd_seek(fd, 0, wasip1::WHENCE_CUR).expect_err("seek on a directory"),
        wasip1::ERRNO_BADF
    );

    // Clean up.
    wasip1::fd_close(fd).expect("failed to close fd");
    wasip1::path_remove_directory(dir_fd, "dir").expect("failed to remove dir");
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
    unsafe { test_directory_seek(dir_fd) }
}
