use libc;
use more_asserts::assert_gt;
use std::{env, process};
use wasi::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_file, close_fd, create_dir, create_file};
use wasi_tests::wasi_wrappers::{wasi_path_open, wasi_path_remove_directory, wasi_path_symlink};

unsafe fn test_nofollow_errors(dir_fd: wasi_unstable::Fd) {
    // Create a directory for the symlink to point to.
    create_dir(dir_fd, "target");

    // Create a symlink.
    assert!(
        wasi_path_symlink("target", dir_fd, "symlink").is_ok(),
        "creating a symlink"
    );

    // Try to open it as a directory with O_NOFOLLOW again.
    let mut file_fd: wasi_unstable::Fd = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(
        dir_fd,
        0,
        "symlink",
        wasi_unstable::O_DIRECTORY,
        0,
        0,
        0,
        &mut file_fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ELOOP,
        "opening a directory symlink as a directory",
    );
    assert_eq!(
        file_fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Try to open it with just O_NOFOLLOW.
    status = wasi_path_open(dir_fd, 0, "symlink", 0, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ELOOP,
        "opening a symlink with O_NOFOLLOW should return ELOOP",
    );
    assert_eq!(
        file_fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Try to open it as a directory without O_NOFOLLOW.
    status = wasi_path_open(
        dir_fd,
        wasi_unstable::LOOKUP_SYMLINK_FOLLOW,
        "symlink",
        wasi_unstable::O_DIRECTORY,
        0,
        0,
        0,
        &mut file_fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a symlink as a directory"
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );
    close_fd(file_fd);

    // Replace the target directory with a file.
    cleanup_file(dir_fd, "symlink");

    assert!(
        wasi_path_remove_directory(dir_fd, "target").is_ok(),
        "remove_directory on a directory should succeed"
    );
    create_file(dir_fd, "target");

    assert!(
        wasi_path_symlink("target", dir_fd, "symlink").is_ok(),
        "creating a symlink"
    );

    // Try to open it as a directory with O_NOFOLLOW again.
    status = wasi_path_open(
        dir_fd,
        0,
        "symlink",
        wasi_unstable::O_DIRECTORY,
        0,
        0,
        0,
        &mut file_fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ELOOP,
        "opening a directory symlink as a directory",
    );
    assert_eq!(
        file_fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Try to open it with just O_NOFOLLOW.
    status = wasi_path_open(dir_fd, 0, "symlink", 0, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ELOOP,
        "opening a symlink with O_NOFOLLOW should return ELOOP",
    );
    assert_eq!(
        file_fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Try to open it as a directory without O_NOFOLLOW.
    status = wasi_path_open(
        dir_fd,
        wasi_unstable::LOOKUP_SYMLINK_FOLLOW,
        "symlink",
        wasi_unstable::O_DIRECTORY,
        0,
        0,
        0,
        &mut file_fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ENOTDIR,
        "opening a symlink to a file as a directory",
    );
    assert_eq!(
        file_fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Clean up.
    cleanup_file(dir_fd, "target");
    cleanup_file(dir_fd, "symlink");
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
    unsafe { test_nofollow_errors(dir_fd) }
}
