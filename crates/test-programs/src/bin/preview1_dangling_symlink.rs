use std::{env, process};
use test_programs::preview1::{assert_errno, config, open_scratch_directory};

unsafe fn test_dangling_symlink(dir_fd: wasi::Fd) {
    if config().support_dangling_filesystem() {
        // First create a dangling symlink.
        wasi::path_symlink("target", dir_fd, "symlink").expect("creating a symlink");

        // Try to open it as a directory with O_NOFOLLOW.
        assert_errno!(
            wasi::path_open(dir_fd, 0, "symlink", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
                .expect_err("opening a dangling symlink as a directory"),
            wasi::ERRNO_NOTDIR,
            wasi::ERRNO_LOOP,
            wasi::ERRNO_NOENT
        );

        // Try to open it as a file with O_NOFOLLOW.
        assert_errno!(
            wasi::path_open(dir_fd, 0, "symlink", 0, 0, 0, 0)
                .expect_err("opening a dangling symlink as a file"),
            wasi::ERRNO_LOOP,
            wasi::ERRNO_NOENT
        );

        // Clean up.
        wasi::path_unlink_file(dir_fd, "symlink").expect("failed to remove file");
    }
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
    unsafe { test_dangling_symlink(dir_fd) }
}
