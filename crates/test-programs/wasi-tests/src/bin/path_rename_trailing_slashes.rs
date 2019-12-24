use std::{env, process};
use wasi_tests::{create_file, open_scratch_directory};

unsafe fn test_path_rename_trailing_slashes(dir_fd: wasi::Fd) {
    // Test renaming a file with a trailing slash in the name.
    create_file(dir_fd, "source");

    assert_eq!(
        wasi::path_rename(dir_fd, "source/", dir_fd, "target")
            .expect_err("renaming a file with a trailing slash in the source name should fail")
            .raw_error(),
        wasi::ERRNO_NOTDIR,
        "errno should be ERRNO_NOTDIR"
    );
    assert_eq!(
        wasi::path_rename(dir_fd, "source", dir_fd, "target/")
            .expect_err("renaming a file with a trailing slash in the destination name should fail")
            .raw_error(),
        wasi::ERRNO_NOTDIR,
        "errno should be ERRNO_NOTDIR"
    );
    assert_eq!(
        wasi::path_rename(dir_fd, "source/", dir_fd, "target/")
            .expect_err("renaming a file with a trailing slash in the source and destination names should fail")
            .raw_error(),
        wasi::ERRNO_NOTDIR,
        "errno should be ERRNO_NOTDIR"
    );
    wasi::path_unlink_file(dir_fd, "source").expect("removing a file");

    // Test renaming a directory with a trailing slash in the name.
    wasi::path_create_directory(dir_fd, "source").expect("creating a directory");
    wasi::path_rename(dir_fd, "source/", dir_fd, "target")
        .expect("renaming a directory with a trailing slash in the source name");
    wasi::path_rename(dir_fd, "target", dir_fd, "source/")
        .expect("renaming a directory with a trailing slash in the destination name");
    wasi::path_rename(dir_fd, "source/", dir_fd, "target/")
        .expect("renaming a directory with a trailing slash in the source and destination names");
    wasi::path_remove_directory(dir_fd, "target").expect("removing a directory");
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
    unsafe { test_path_rename_trailing_slashes(dir_fd) }
}
