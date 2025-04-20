#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

use std::{env, process};
use test_programs::preview1::{config, open_scratch_directory};

unsafe fn test_dangling_fd(dir_fd: wasip1::Fd) {
    if config().support_dangling_filesystem() {
        // Create a file, open it, delete it without closing the handle,
        // and then try creating it again
        let fd = wasip1::path_open(dir_fd, 0, "file", wasip1::OFLAGS_CREAT, 0, 0, 0).unwrap();
        wasip1::fd_close(fd).unwrap();
        let file_fd = wasip1::path_open(dir_fd, 0, "file", 0, 0, 0, 0).expect("failed to open");
        assert!(
            file_fd > libc::STDERR_FILENO as wasip1::Fd,
            "file descriptor range check",
        );
        wasip1::path_unlink_file(dir_fd, "file").expect("failed to unlink");
        let fd = wasip1::path_open(dir_fd, 0, "file", wasip1::OFLAGS_CREAT, 0, 0, 0).unwrap();
        wasip1::fd_close(fd).unwrap();

        // Now, repeat the same process but for a directory
        wasip1::path_create_directory(dir_fd, "subdir").expect("failed to create dir");
        let subdir_fd = wasip1::path_open(dir_fd, 0, "subdir", wasip1::OFLAGS_DIRECTORY, 0, 0, 0)
            .expect("failed to open dir");
        assert!(
            subdir_fd > libc::STDERR_FILENO as wasip1::Fd,
            "file descriptor range check",
        );
        wasip1::path_remove_directory(dir_fd, "subdir").expect("failed to remove dir 2");
        wasip1::path_create_directory(dir_fd, "subdir").expect("failed to create dir 2");
    }
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
    unsafe { test_dangling_fd(dir_fd) }
}
