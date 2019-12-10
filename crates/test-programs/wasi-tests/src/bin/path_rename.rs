use more_asserts::assert_gt;
use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_dir, cleanup_file, close_fd, create_dir, create_file};
use wasi_tests::wasi_wrappers::{wasi_path_open, wasi_path_rename};

unsafe fn test_path_rename(dir_fd: wasi_unstable::Fd) {
    // First, try renaming a dir to nonexistent path
    // Create source directory
    create_dir(dir_fd, "source");

    // Try renaming the directory
    assert!(
        wasi_path_rename(dir_fd, "source", dir_fd, "target").is_ok(),
        "renaming a directory"
    );

    // Check that source directory doesn't exist anymore
    let mut fd: wasi_unstable::Fd = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(
        dir_fd,
        0,
        "source",
        wasi_unstable::O_DIRECTORY,
        0,
        0,
        0,
        &mut fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ENOENT,
        "opening a nonexistent path as a directory"
    );
    assert_eq!(
        fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Check that target directory exists
    status = wasi_path_open(
        dir_fd,
        0,
        "target",
        wasi_unstable::O_DIRECTORY,
        0,
        0,
        0,
        &mut fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening renamed path as a directory"
    );
    assert_gt!(
        fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );

    close_fd(fd);
    cleanup_dir(dir_fd, "target");

    // Now, try renaming renaming a dir to existing empty dir
    create_dir(dir_fd, "source");
    create_dir(dir_fd, "target");

    assert!(
        wasi_path_rename(dir_fd, "source", dir_fd, "target").is_ok(),
        "renaming a directory"
    );

    // Check that source directory doesn't exist anymore
    fd = wasi_unstable::Fd::max_value() - 1;
    status = wasi_path_open(
        dir_fd,
        0,
        "source",
        wasi_unstable::O_DIRECTORY,
        0,
        0,
        0,
        &mut fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ENOENT,
        "opening a nonexistent path as a directory"
    );
    assert_eq!(
        fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Check that target directory exists
    status = wasi_path_open(
        dir_fd,
        0,
        "target",
        wasi_unstable::O_DIRECTORY,
        0,
        0,
        0,
        &mut fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening renamed path as a directory"
    );
    assert_gt!(
        fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );

    close_fd(fd);
    cleanup_dir(dir_fd, "target");

    // Now, try renaming a dir to existing non-empty dir
    create_dir(dir_fd, "source");
    create_dir(dir_fd, "target");
    create_file(dir_fd, "target/file");

    assert_eq!(
        wasi_path_rename(dir_fd, "source", dir_fd, "target"),
        Err(wasi_unstable::ENOTEMPTY),
        "renaming directory to a nonempty directory"
    );

    // Try renaming dir to a file
    assert_eq!(
        wasi_path_rename(dir_fd, "source", dir_fd, "target/file"),
        Err(wasi_unstable::ENOTDIR),
        "renaming directory to a file"
    );

    cleanup_file(dir_fd, "target/file");
    cleanup_dir(dir_fd, "target");
    cleanup_dir(dir_fd, "source");

    // Now, try renaming a file to a nonexistent path
    create_file(dir_fd, "source");

    assert!(
        wasi_path_rename(dir_fd, "source", dir_fd, "target").is_ok(),
        "renaming a file"
    );

    // Check that source file doesn't exist anymore
    fd = wasi_unstable::Fd::max_value() - 1;
    status = wasi_path_open(dir_fd, 0, "source", 0, 0, 0, 0, &mut fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ENOENT,
        "opening a nonexistent path"
    );
    assert_eq!(
        fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Check that target file exists
    status = wasi_path_open(dir_fd, 0, "target", 0, 0, 0, 0, &mut fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening renamed path"
    );
    assert_gt!(
        fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );

    close_fd(fd);
    cleanup_file(dir_fd, "target");

    // Now, try renaming file to an existing file
    create_file(dir_fd, "source");
    create_file(dir_fd, "target");

    assert!(
        wasi_path_rename(dir_fd, "source", dir_fd, "target").is_ok(),
        "renaming file to another existing file"
    );

    // Check that source file doesn't exist anymore
    fd = wasi_unstable::Fd::max_value() - 1;
    status = wasi_path_open(dir_fd, 0, "source", 0, 0, 0, 0, &mut fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ENOENT,
        "opening a nonexistent path"
    );
    assert_eq!(
        fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Check that target file exists
    status = wasi_path_open(dir_fd, 0, "target", 0, 0, 0, 0, &mut fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening renamed path"
    );
    assert_gt!(
        fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );

    close_fd(fd);
    cleanup_file(dir_fd, "target");

    // Try renaming to an (empty) directory instead
    create_file(dir_fd, "source");
    create_dir(dir_fd, "target");

    assert_eq!(
        wasi_path_rename(dir_fd, "source", dir_fd, "target"),
        Err(wasi_unstable::EISDIR),
        "renaming file to existing directory"
    );

    cleanup_dir(dir_fd, "target");
    cleanup_file(dir_fd, "source");
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
    unsafe { test_path_rename(dir_fd) }
}
