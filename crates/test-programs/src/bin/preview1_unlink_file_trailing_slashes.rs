use std::{env, process};
use test_programs::preview1::{assert_errno, create_file, open_scratch_directory};

unsafe fn test_unlink_file_trailing_slashes(dir_fd: wasip1::Fd) {
    // Create a directory in the scratch directory.
    wasip1::path_create_directory(dir_fd, "dir").expect("creating a directory");

    // Test that unlinking it fails.
    assert_errno!(
        wasip1::path_unlink_file(dir_fd, "dir")
            .expect_err("unlink_file on a directory should fail"),
        macos => wasip1::ERRNO_PERM,
        unix => wasip1::ERRNO_ISDIR,
        windows => wasip1::ERRNO_ACCES
    );

    // Test that unlinking it with a trailing flash fails.
    assert_errno!(
        wasip1::path_unlink_file(dir_fd, "dir/")
            .expect_err("unlink_file on a directory should fail"),
        macos => wasip1::ERRNO_PERM,
        unix => wasip1::ERRNO_ISDIR,
        windows => wasip1::ERRNO_ACCES
    );

    // Clean up.
    wasip1::path_remove_directory(dir_fd, "dir").expect("removing a directory");

    // Create a temporary file.
    create_file(dir_fd, "file");

    // Test that unlinking it with a trailing flash fails.
    assert_errno!(
        wasip1::path_unlink_file(dir_fd, "file/")
            .expect_err("unlink_file with a trailing slash should fail"),
        wasip1::ERRNO_NOTDIR
    );

    // Test that unlinking it with no trailing flash succeeds.
    wasip1::path_unlink_file(dir_fd, "file")
        .expect("unlink_file with no trailing slash should succeed");
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
    unsafe { test_unlink_file_trailing_slashes(dir_fd) }
}
