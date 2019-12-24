use libc;
use more_asserts::assert_gt;
use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn test_nofollow_errors(dir_fd: wasi::Fd) {
    // Create a directory for the symlink to point to.
    wasi::path_create_directory(dir_fd, "target").expect("creating a dir");

    // Create a symlink.
    wasi::path_symlink("target", dir_fd, "symlink").expect("creating a symlink");

    // Try to open it as a directory with O_NOFOLLOW again.
    assert_eq!(
        wasi::path_open(dir_fd, 0, "symlink", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
            .expect_err("opening a directory symlink as a directory should fail")
            .raw_error(),
        wasi::ERRNO_LOOP,
        "errno should be ERRNO_LOOP",
    );

    // Try to open it with just O_NOFOLLOW.
    assert_eq!(
        wasi::path_open(dir_fd, 0, "symlink", 0, 0, 0, 0)
            .expect_err("opening a symlink with O_NOFOLLOW should fail")
            .raw_error(),
        wasi::ERRNO_LOOP,
        "errno should be ERRNO_LOOP",
    );

    // Try to open it as a directory without O_NOFOLLOW.
    let file_fd = wasi::path_open(
        dir_fd,
        wasi::LOOKUPFLAGS_SYMLINK_FOLLOW,
        "symlink",
        wasi::OFLAGS_DIRECTORY,
        0,
        0,
        0,
    )
    .expect("opening a symlink as a directory");
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );
    wasi::fd_close(file_fd).expect("closing a file");

    // Replace the target directory with a file.
    wasi::path_unlink_file(dir_fd, "symlink").expect("removing a file");
    wasi::path_remove_directory(dir_fd, "target")
        .expect("remove_directory on a directory should succeed");

    let file_fd =
        wasi::path_open(dir_fd, 0, "target", wasi::OFLAGS_CREAT, 0, 0, 0).expect("creating a file");
    wasi::fd_close(file_fd).expect("closing a file");
    wasi::path_symlink("target", dir_fd, "symlink").expect("creating a symlink");

    // Try to open it as a directory with O_NOFOLLOW again.
    assert_eq!(
        wasi::path_open(dir_fd, 0, "symlink", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
            .expect_err("opening a directory symlink as a directory should fail")
            .raw_error(),
        wasi::ERRNO_LOOP,
        "errno should be ERRNO_LOOP",
    );

    // Try to open it with just O_NOFOLLOW.
    assert_eq!(
        wasi::path_open(dir_fd, 0, "symlink", 0, 0, 0, 0)
            .expect_err("opening a symlink with NOFOLLOW should fail")
            .raw_error(),
        wasi::ERRNO_LOOP,
        "errno should be ERRNO_LOOP",
    );

    // Try to open it as a directory without O_NOFOLLOW.
    assert_eq!(
        wasi::path_open(
            dir_fd,
            wasi::LOOKUPFLAGS_SYMLINK_FOLLOW,
            "symlink",
            wasi::OFLAGS_DIRECTORY,
            0,
            0,
            0,
        )
        .expect_err("opening a symlink to a file as a directory")
        .raw_error(),
        wasi::ERRNO_NOTDIR,
        "errno should be ERRNO_NOTDIR",
    );

    // Clean up.
    wasi::path_unlink_file(dir_fd, "target").expect("removing a file");
    wasi::path_unlink_file(dir_fd, "symlink").expect("removing a file");
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
