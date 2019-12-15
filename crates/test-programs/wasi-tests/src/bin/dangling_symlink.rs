use std::{env, process};
use wasi_tests::open_scratch_directory_new;

unsafe fn test_dangling_symlink(dir_fd: wasi::Fd) {
    // First create a dangling symlink.
    assert!(
        wasi::path_symlink("target", dir_fd, "symlink").is_ok(),
        "creating a symlink"
    );

    // Try to open it as a directory with O_NOFOLLOW.
    let status = wasi::path_open(dir_fd, 0, "symlink", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
        .err()
        .expect("failed to open symlink");
    assert_eq!(
        status.raw_error(),
        wasi::ERRNO_LOOP,
        "opening a dangling symlink as a directory",
    );

    // Clean up.
    wasi::path_unlink_file(dir_fd, "symlink").expect("failed to remove file");
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
    let dir_fd = match open_scratch_directory_new(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe { test_dangling_symlink(dir_fd) }
}
