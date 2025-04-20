#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

use std::{env, process};
use test_programs::preview1::{assert_errno, open_scratch_directory};

unsafe fn test_remove_nonempty_directory(dir_fd: wasip1::Fd) {
    // Create a directory in the scratch directory.
    wasip1::path_create_directory(dir_fd, "dir").expect("creating a directory");

    // Create a directory in the directory we just created.
    wasip1::path_create_directory(dir_fd, "dir/nested").expect("creating a subdirectory");

    // Test that attempting to unlink the first directory returns the expected error code.
    assert_errno!(
        wasip1::path_remove_directory(dir_fd, "dir")
            .expect_err("remove_directory on a directory should return ENOTEMPTY"),
        wasip1::ERRNO_NOTEMPTY
    );

    // Removing the directories.
    wasip1::path_remove_directory(dir_fd, "dir/nested")
        .expect("remove_directory on a nested directory should succeed");
    wasip1::path_remove_directory(dir_fd, "dir").expect("removing a directory");
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
    unsafe { test_remove_nonempty_directory(dir_fd) }
}
