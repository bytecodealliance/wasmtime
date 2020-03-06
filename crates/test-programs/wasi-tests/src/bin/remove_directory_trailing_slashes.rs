use std::{env, process};
use wasi_tests::{create_file, open_scratch_directory};

unsafe fn test_remove_directory_trailing_slashes(dir_fd: wasi::Fd) {
    // Create a directory in the scratch directory.
    wasi::path_create_directory(dir_fd, "dir").expect("creating a directory");

    // Test that removing it succeeds.
    wasi::path_remove_directory(dir_fd, "dir")
        .expect("remove_directory on a directory should succeed");

    wasi::path_create_directory(dir_fd, "dir").expect("creating a directory");

    // Test that removing it with a trailing slash succeeds.
    wasi::path_remove_directory(dir_fd, "dir/")
        .expect("remove_directory with a trailing slash on a directory should succeed");

    // Create a temporary file.
    create_file(dir_fd, "file");

    // Test that removing it with no trailing slash fails.
    assert_eq!(
        wasi::path_remove_directory(dir_fd, "file")
            .expect_err("remove_directory without a trailing slash on a file should fail")
            .raw_error(),
        wasi::ERRNO_NOTDIR,
        "errno should be ERRNO_NOTDIR"
    );

    // Test that removing it with a trailing slash fails.
    assert_eq!(
        wasi::path_remove_directory(dir_fd, "file/")
            .expect_err("remove_directory with a trailing slash on a file should fail")
            .raw_error(),
        wasi::ERRNO_NOTDIR,
        "errno should be ERRNO_NOTDIR"
    );

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
    unsafe { test_remove_directory_trailing_slashes(dir_fd) }
}
