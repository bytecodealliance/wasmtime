use std::{env, process};
use test_programs::preview1::{assert_errno, config, create_file, open_scratch_directory};

unsafe fn test_path_rename(dir_fd: wasi::Fd) {
    // First, try renaming a dir to nonexistent path
    // Create source directory
    wasi::path_create_directory(dir_fd, "source").expect("creating a directory");

    // Try renaming the directory
    wasi::path_rename(dir_fd, "source", dir_fd, "target").expect("renaming a directory");

    // Check that source directory doesn't exist anymore
    assert_errno!(
        wasi::path_open(dir_fd, 0, "source", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
            .expect_err("opening a nonexistent path as a directory should fail"),
        wasi::ERRNO_NOENT
    );

    // Check that target directory exists
    let mut fd = wasi::path_open(dir_fd, 0, "target", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
        .expect("opening renamed path as a directory");
    assert!(
        fd > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    wasi::fd_close(fd).expect("closing a file");
    wasi::path_remove_directory(dir_fd, "target").expect("removing a directory");

    // Yes, renaming a dir to an empty dir is a property guaranteed by rename(2)
    // and its fairly important that it is atomic.
    // But, we haven't found a way to emulate it on windows. So, sometimes this
    // behavior is just hosed. Sorry.
    if config().support_rename_dir_to_empty_dir() {
        // Now, try renaming renaming a dir to existing empty dir
        wasi::path_create_directory(dir_fd, "source").expect("creating a directory");
        wasi::path_create_directory(dir_fd, "target").expect("creating a directory");
        wasi::path_rename(dir_fd, "source", dir_fd, "target").expect("renaming a directory");

        // Check that source directory doesn't exist anymore
        assert_errno!(
            wasi::path_open(dir_fd, 0, "source", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
                .expect_err("opening a nonexistent path as a directory"),
            wasi::ERRNO_NOENT
        );

        // Check that target directory exists
        fd = wasi::path_open(dir_fd, 0, "target", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
            .expect("opening renamed path as a directory");
        assert!(
            fd > libc::STDERR_FILENO as wasi::Fd,
            "file descriptor range check",
        );

        wasi::fd_close(fd).expect("closing a file");
        wasi::path_remove_directory(dir_fd, "target").expect("removing a directory");
    } else {
        wasi::path_create_directory(dir_fd, "source").expect("creating a directory");
        wasi::path_create_directory(dir_fd, "target").expect("creating a directory");
        wasi::path_rename(dir_fd, "source", dir_fd, "target")
            .expect_err("windows does not support renaming a directory to an empty directory");
        wasi::path_remove_directory(dir_fd, "target").expect("removing a directory");
        wasi::path_remove_directory(dir_fd, "source").expect("removing a directory");
    }

    // Now, try renaming a dir to existing non-empty dir
    wasi::path_create_directory(dir_fd, "source").expect("creating a directory");
    wasi::path_create_directory(dir_fd, "target").expect("creating a directory");
    create_file(dir_fd, "target/file");

    assert_errno!(
        wasi::path_rename(dir_fd, "source", dir_fd, "target")
            .expect_err("renaming directory to a nonempty directory"),
        windows => wasi::ERRNO_ACCES,
        unix => wasi::ERRNO_NOTEMPTY
    );

    // This is technically a different property, but the root of these divergent behaviors is in
    // the semantics that windows gives us around renaming directories. So, it lives under the same
    // flag.
    if config().support_rename_dir_to_empty_dir() {
        // Try renaming dir to a file
        assert_errno!(
            wasi::path_rename(dir_fd, "source", dir_fd, "target/file")
                .expect_err("renaming a directory to a file"),
            wasi::ERRNO_NOTDIR
        );
        wasi::path_unlink_file(dir_fd, "target/file").expect("removing a file");
        wasi::path_remove_directory(dir_fd, "source").expect("removing a directory");
    } else {
        // Windows will let you erase a file by renaming a directory to it.
        // WASI users can't depend on this error getting caught to prevent data loss.
        wasi::path_rename(dir_fd, "source", dir_fd, "target/file")
            .expect("windows happens to support renaming a directory to a file");
        wasi::path_remove_directory(dir_fd, "target/file").expect("removing a file");
    }
    wasi::path_remove_directory(dir_fd, "target").expect("removing a directory");

    // Now, try renaming a file to a nonexistent path
    create_file(dir_fd, "source");
    wasi::path_rename(dir_fd, "source", dir_fd, "target").expect("renaming a file");

    // Check that source file doesn't exist anymore
    assert_errno!(
        wasi::path_open(dir_fd, 0, "source", 0, 0, 0, 0)
            .expect_err("opening a nonexistent path should fail"),
        wasi::ERRNO_NOENT
    );

    // Check that target file exists
    fd = wasi::path_open(dir_fd, 0, "target", 0, 0, 0, 0).expect("opening renamed path");
    assert!(
        fd > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    wasi::fd_close(fd).expect("closing a file");
    wasi::path_unlink_file(dir_fd, "target").expect("removing a file");

    // Now, try renaming file to an existing file
    create_file(dir_fd, "source");
    create_file(dir_fd, "target");

    wasi::path_rename(dir_fd, "source", dir_fd, "target")
        .expect("renaming file to another existing file");

    // Check that source file doesn't exist anymore
    assert_errno!(
        wasi::path_open(dir_fd, 0, "source", 0, 0, 0, 0).expect_err("opening a nonexistent path"),
        wasi::ERRNO_NOENT
    );

    // Check that target file exists
    fd = wasi::path_open(dir_fd, 0, "target", 0, 0, 0, 0).expect("opening renamed path");
    assert!(
        fd > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    wasi::fd_close(fd).expect("closing a file");
    wasi::path_unlink_file(dir_fd, "target").expect("removing a file");

    // Try renaming to an (empty) directory instead
    create_file(dir_fd, "source");
    wasi::path_create_directory(dir_fd, "target").expect("creating a directory");

    assert_errno!(
        wasi::path_rename(dir_fd, "source", dir_fd, "target")
            .expect_err("renaming a file to existing directory should fail"),
        windows => wasi::ERRNO_ACCES,
        unix => wasi::ERRNO_ISDIR
    );

    wasi::path_remove_directory(dir_fd, "target").expect("removing a directory");
    wasi::path_unlink_file(dir_fd, "source").expect("removing a file");
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
