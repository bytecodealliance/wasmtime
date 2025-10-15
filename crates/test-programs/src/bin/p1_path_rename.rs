#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

use std::{env, process};
use test_programs::preview1::{TestConfig, assert_errno, create_file, open_scratch_directory};

unsafe fn test_path_rename(dir_fd: wasip1::Fd) {
    // First, try renaming a dir to nonexistent path
    // Create source directory
    wasip1::path_create_directory(dir_fd, "source").expect("creating a directory");

    // Try renaming the directory
    wasip1::path_rename(dir_fd, "source", dir_fd, "target").expect("renaming a directory");

    // Check that source directory doesn't exist anymore
    assert_errno!(
        wasip1::path_open(dir_fd, 0, "source", wasip1::OFLAGS_DIRECTORY, 0, 0, 0)
            .expect_err("opening a nonexistent path as a directory should fail"),
        wasip1::ERRNO_NOENT
    );

    // Check that target directory exists
    let mut fd = wasip1::path_open(dir_fd, 0, "target", wasip1::OFLAGS_DIRECTORY, 0, 0, 0)
        .expect("opening renamed path as a directory");
    assert!(
        fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    wasip1::fd_close(fd).expect("closing a file");
    wasip1::path_remove_directory(dir_fd, "target").expect("removing a directory");

    // Now, try renaming renaming a dir to existing empty dir
    wasip1::path_create_directory(dir_fd, "source").expect("creating a directory");
    wasip1::path_create_directory(dir_fd, "target").expect("creating a directory");
    wasip1::path_rename(dir_fd, "source", dir_fd, "target").expect("renaming a directory");

    // Check that source directory doesn't exist anymore
    assert_errno!(
        wasip1::path_open(dir_fd, 0, "source", wasip1::OFLAGS_DIRECTORY, 0, 0, 0)
            .expect_err("opening a nonexistent path as a directory"),
        wasip1::ERRNO_NOENT
    );

    // Check that target directory exists
    fd = wasip1::path_open(dir_fd, 0, "target", wasip1::OFLAGS_DIRECTORY, 0, 0, 0)
        .expect("opening renamed path as a directory");
    assert!(
        fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    wasip1::fd_close(fd).expect("closing a file");
    wasip1::path_remove_directory(dir_fd, "target").expect("removing a directory");

    // Now, try renaming a dir to existing non-empty dir
    wasip1::path_create_directory(dir_fd, "source").expect("creating a directory");
    wasip1::path_create_directory(dir_fd, "target").expect("creating a directory");
    create_file(dir_fd, "target/file");

    assert_errno!(
        wasip1::path_rename(dir_fd, "source", dir_fd, "target")
            .expect_err("renaming directory to a nonempty directory"),
        wasip1::ERRNO_NOTEMPTY
    );

    // Try renaming dir to a file
    if TestConfig::from_env().support_rename_dir_onto_file() {
        wasip1::path_rename(dir_fd, "source", dir_fd, "target/file").unwrap();
        wasip1::path_remove_directory(dir_fd, "target/file").expect("removing a directory");
    } else {
        assert_errno!(
            wasip1::path_rename(dir_fd, "source", dir_fd, "target/file")
                .expect_err("renaming a directory to a file"),
            wasip1::ERRNO_NOTDIR
        );
        wasip1::path_unlink_file(dir_fd, "target/file").expect("removing a file");
        wasip1::path_remove_directory(dir_fd, "source").expect("removing a directory");
    }
    wasip1::path_remove_directory(dir_fd, "target").expect("removing a directory");

    // Now, try renaming a file to a nonexistent path
    create_file(dir_fd, "source");
    wasip1::path_rename(dir_fd, "source", dir_fd, "target").expect("renaming a file");

    // Check that source file doesn't exist anymore
    assert_errno!(
        wasip1::path_open(dir_fd, 0, "source", 0, 0, 0, 0)
            .expect_err("opening a nonexistent path should fail"),
        wasip1::ERRNO_NOENT
    );

    // Check that target file exists
    fd = wasip1::path_open(dir_fd, 0, "target", 0, 0, 0, 0).expect("opening renamed path");
    assert!(
        fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    wasip1::fd_close(fd).expect("closing a file");
    wasip1::path_unlink_file(dir_fd, "target").expect("removing a file");

    // Now, try renaming file to an existing file
    create_file(dir_fd, "source");
    create_file(dir_fd, "target");

    wasip1::path_rename(dir_fd, "source", dir_fd, "target")
        .expect("renaming file to another existing file");

    // Check that source file doesn't exist anymore
    assert_errno!(
        wasip1::path_open(dir_fd, 0, "source", 0, 0, 0, 0).expect_err("opening a nonexistent path"),
        wasip1::ERRNO_NOENT
    );

    // Check that target file exists
    fd = wasip1::path_open(dir_fd, 0, "target", 0, 0, 0, 0).expect("opening renamed path");
    assert!(
        fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    wasip1::fd_close(fd).expect("closing a file");
    wasip1::path_unlink_file(dir_fd, "target").expect("removing a file");

    // Try renaming to an (empty) directory instead
    create_file(dir_fd, "source");
    wasip1::path_create_directory(dir_fd, "target").expect("creating a directory");

    assert_errno!(
        wasip1::path_rename(dir_fd, "source", dir_fd, "target")
            .expect_err("renaming a file to existing directory should fail"),
        windows => wasip1::ERRNO_ACCES,
        unix => wasip1::ERRNO_ISDIR
    );

    wasip1::path_remove_directory(dir_fd, "target").expect("removing a directory");
    wasip1::path_unlink_file(dir_fd, "source").expect("removing a file");
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
    unsafe { test_path_rename(dir_fd) }
}
