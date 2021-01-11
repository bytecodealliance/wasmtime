use std::{env, process};
use wasi_tests::{create_file, open_scratch_directory};

unsafe fn test_path_symlink_trailing_slashes(dir_fd: wasi::Fd) {
    // Link destination shouldn't end with a slash.
    assert_eq!(
        wasi::path_symlink("source", dir_fd, "target/")
            .expect_err("link destination ending with a slash should fail")
            .raw_error(),
        wasi::ERRNO_NOENT,
        "errno should be ERRNO_NOENT"
    );

    // Without the trailing slash, this should succeed.
    wasi::path_symlink("source", dir_fd, "target").expect("link destination ending with a slash");
    wasi::path_unlink_file(dir_fd, "target").expect("removing a file");

    // Link destination already exists, target has trailing slash.
    wasi::path_create_directory(dir_fd, "target").expect("creating a directory");
    assert_eq!(
        wasi::path_symlink("source", dir_fd, "target/")
            .expect_err("link destination already exists")
            .raw_error(),
        wasi::ERRNO_EXIST,
        "errno should be ERRNO_EXIST"
    );
    wasi::path_remove_directory(dir_fd, "target").expect("removing a directory");

    // Link destination already exists, target has no trailing slash.
    wasi::path_create_directory(dir_fd, "target").expect("creating a directory");
    assert_eq!(
        wasi::path_symlink("source", dir_fd, "target")
            .expect_err("link destination already exists")
            .raw_error(),
        wasi::ERRNO_EXIST,
        "errno should be ERRNO_EXIST"
    );
    wasi::path_remove_directory(dir_fd, "target").expect("removing a directory");

    // Link destination already exists, target has trailing slash.
    create_file(dir_fd, "target");

    let dir_symlink_errno = wasi::path_symlink("source", dir_fd, "target/")
        .expect_err("link destination already exists")
        .raw_error();
    assert!(
        dir_symlink_errno == wasi::ERRNO_EXIST || dir_symlink_errno == wasi::ERRNO_NOTDIR,
        "errno should be ERRNO_EXIST or ERRNO_NOTDIR"
    );
    wasi::path_unlink_file(dir_fd, "target").expect("removing a file");

    // Link destination already exists, target has no trailing slash.
    create_file(dir_fd, "target");

    assert_eq!(
        wasi::path_symlink("source", dir_fd, "target")
            .expect_err("link destination already exists")
            .raw_error(),
        wasi::ERRNO_EXIST,
        "errno should be ERRNO_EXIST"
    );
    wasi::path_unlink_file(dir_fd, "target").expect("removing a file");
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
    unsafe { test_path_symlink_trailing_slashes(dir_fd) }
}
