use std::{env, process};
use test_programs::preview1::{assert_errno, config, open_scratch_directory};

unsafe fn test_symlink_loop(dir_fd: wasip1::Fd) {
    if config().support_dangling_filesystem() {
        // Create a self-referencing symlink.
        wasip1::path_symlink("symlink", dir_fd, "symlink").expect("creating a symlink");

        // Try to open it.
        assert_errno!(
            wasip1::path_open(dir_fd, 0, "symlink", 0, 0, 0, 0)
                .expect_err("opening a self-referencing symlink"),
            wasip1::ERRNO_LOOP
        );

        // Clean up.
        wasip1::path_unlink_file(dir_fd, "symlink").expect("removing a file");
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
    unsafe { test_symlink_loop(dir_fd) }
}
