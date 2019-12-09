use more_asserts::assert_gt;
use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_dir, cleanup_file, create_dir, create_file};
use wasi_tests::wasi_wrappers::wasi_path_open;

unsafe fn test_dangling_fd(dir_fd: wasi_unstable::Fd) {
    // Create a file, open it, delete it without closing the handle,
    // and then try creating it again
    create_file(dir_fd, "file");
    let mut file_fd = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(dir_fd, 0, "file", 0, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a file",
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );
    cleanup_file(dir_fd, "file");
    create_file(dir_fd, "file");

    // Now, repeat the same process but for a directory
    create_dir(dir_fd, "subdir");
    let mut subdir_fd = wasi_unstable::Fd::max_value() - 1;
    status = wasi_path_open(
        dir_fd,
        0,
        "subdir",
        wasi_unstable::O_DIRECTORY,
        0,
        0,
        0,
        &mut subdir_fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a directory",
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );
    cleanup_dir(dir_fd, "subdir");
    create_dir(dir_fd, "subdir");
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
    unsafe { test_dangling_fd(dir_fd) }
}
