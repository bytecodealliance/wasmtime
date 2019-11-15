use std::{env, process};
use wasi::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_file, create_dir, create_file};
use wasi_tests::wasi_wrappers::wasi_path_remove_directory;

unsafe fn test_remove_directory_trailing_slashes(dir_fd: wasi_unstable::Fd) {
    // Create a directory in the scratch directory.
    create_dir(dir_fd, "dir");

    // Test that removing it succeeds.
    assert_eq!(
        wasi_path_remove_directory(dir_fd, "dir"),
        Ok(()),
        "remove_directory on a directory should succeed"
    );

    create_dir(dir_fd, "dir");

    // Test that removing it with a trailing flash succeeds.
    assert_eq!(
        wasi_path_remove_directory(dir_fd, "dir/"),
        Ok(()),
        "remove_directory with a trailing slash on a directory should succeed"
    );

    // Create a temporary file.
    create_file(dir_fd, "file");

    // Test that removing it with no trailing flash fails.
    assert_eq!(
        wasi_path_remove_directory(dir_fd, "file"),
        Err(wasi_unstable::ENOTDIR),
        "remove_directory without a trailing slash on a file should fail"
    );

    // Test that removing it with a trailing flash fails.
    assert_eq!(
        wasi_path_remove_directory(dir_fd, "file/"),
        Err(wasi_unstable::ENOTDIR),
        "remove_directory with a trailing slash on a file should fail"
    );

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
    unsafe { test_remove_directory_trailing_slashes(dir_fd) }
}
