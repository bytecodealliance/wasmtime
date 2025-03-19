use std::{env, process};
use test_programs::preview1::{assert_errno, open_scratch_directory};

unsafe fn test_nofollow_errors(dir_fd: wasip1::Fd) {
    // Create a directory for the symlink to point to.
    wasip1::path_create_directory(dir_fd, "target").expect("creating a dir");

    // Create a symlink.
    wasip1::path_symlink("target", dir_fd, "symlink").expect("creating a symlink");

    // Try to open it as a directory with O_NOFOLLOW again.
    assert_errno!(
        wasip1::path_open(dir_fd, 0, "symlink", wasip1::OFLAGS_DIRECTORY, 0, 0, 0)
            .expect_err("opening a directory symlink as a directory should fail"),
        wasip1::ERRNO_LOOP,
        wasip1::ERRNO_NOTDIR
    );

    // Try to open it with just O_NOFOLLOW.
    assert_errno!(
        wasip1::path_open(dir_fd, 0, "symlink", 0, 0, 0, 0)
            .expect_err("opening a symlink with O_NOFOLLOW should fail"),
        wasip1::ERRNO_LOOP,
        wasip1::ERRNO_ACCES
    );

    // Try to open it as a directory without O_NOFOLLOW.
    let file_fd = wasip1::path_open(
        dir_fd,
        wasip1::LOOKUPFLAGS_SYMLINK_FOLLOW,
        "symlink",
        wasip1::OFLAGS_DIRECTORY,
        0,
        0,
        0,
    )
    .expect("opening a symlink as a directory");
    assert!(
        file_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );
    wasip1::fd_close(file_fd).expect("closing a file");

    // Replace the target directory with a file.
    wasip1::path_unlink_file(dir_fd, "symlink").expect("removing a file");
    wasip1::path_remove_directory(dir_fd, "target")
        .expect("remove_directory on a directory should succeed");

    let file_fd = wasip1::path_open(dir_fd, 0, "target", wasip1::OFLAGS_CREAT, 0, 0, 0)
        .expect("creating a file");
    wasip1::fd_close(file_fd).expect("closing a file");
    wasip1::path_symlink("target", dir_fd, "symlink").expect("creating a symlink");

    // Try to open it as a directory with O_NOFOLLOW again.
    assert_errno!(
        wasip1::path_open(dir_fd, 0, "symlink", wasip1::OFLAGS_DIRECTORY, 0, 0, 0)
            .expect_err("opening a directory symlink as a directory should fail"),
        wasip1::ERRNO_LOOP,
        wasip1::ERRNO_NOTDIR
    );

    // Try to open it with just O_NOFOLLOW.
    assert_errno!(
        wasip1::path_open(dir_fd, 0, "symlink", 0, 0, 0, 0)
            .expect_err("opening a symlink with NOFOLLOW should fail"),
        wasip1::ERRNO_LOOP
    );

    // Try to open it as a directory without O_NOFOLLOW.
    assert_errno!(
        wasip1::path_open(
            dir_fd,
            wasip1::LOOKUPFLAGS_SYMLINK_FOLLOW,
            "symlink",
            wasip1::OFLAGS_DIRECTORY,
            0,
            0,
            0,
        )
        .expect_err("opening a symlink to a file as a directory"),
        wasip1::ERRNO_NOTDIR
    );

    // Clean up.
    wasip1::path_unlink_file(dir_fd, "target").expect("removing a file");
    wasip1::path_unlink_file(dir_fd, "symlink").expect("removing a file");
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
    unsafe { test_nofollow_errors(dir_fd) }
}
