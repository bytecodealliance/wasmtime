use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_dir, create_dir, create_file};
use wasi_tests::wasi_wrappers::wasi_path_unlink_file;

unsafe fn test_unlink_file_trailing_slashes(dir_fd: wasi_unstable::Fd) {
    // Create a directory in the scratch directory.
    create_dir(dir_fd, "dir");

    // Test that unlinking it fails.
    assert_eq!(
        wasi_path_unlink_file(dir_fd, "dir"),
        Err(wasi_unstable::EISDIR),
        "unlink_file on a directory should fail"
    );

    // Test that unlinking it with a trailing flash fails.
    assert_eq!(
        wasi_path_unlink_file(dir_fd, "dir/"),
        Err(wasi_unstable::EISDIR),
        "unlink_file on a directory should fail"
    );

    // Clean up.
    cleanup_dir(dir_fd, "dir");

    // Create a temporary file.
    create_file(dir_fd, "file");

    // Test that unlinking it with a trailing flash fails.
    assert_eq!(
        wasi_path_unlink_file(dir_fd, "file/"),
        Err(wasi_unstable::ENOTDIR),
        "unlink_file with a trailing slash should fail"
    );

    // Test that unlinking it with no trailing flash succeeds.
    assert_eq!(
        wasi_path_unlink_file(dir_fd, "file"),
        Ok(()),
        "unlink_file with no trailing slash should succeed"
    );
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
    unsafe { test_unlink_file_trailing_slashes(dir_fd) }
}
