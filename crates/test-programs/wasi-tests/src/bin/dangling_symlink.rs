use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::cleanup_file;
use wasi_tests::wasi_wrappers::{wasi_path_open, wasi_path_symlink};

unsafe fn test_dangling_symlink(dir_fd: wasi_unstable::Fd) {
    // First create a dangling symlink.
    assert!(
        wasi_path_symlink("target", dir_fd, "symlink").is_ok(),
        "creating a symlink"
    );

    // Try to open it as a directory with O_NOFOLLOW.
    let mut file_fd: wasi_unstable::Fd = wasi_unstable::Fd::max_value() - 1;
    let status = wasi_path_open(
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
        "opening a dangling symlink as a directory",
    );
    assert_eq!(
        file_fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Clean up.
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
    unsafe { test_dangling_symlink(dir_fd) }
}
