use std::{env, process};
use test_programs::preview1::{assert_errno, create_file, open_scratch_directory};

unsafe fn test_path_open_create_existing(dir_fd: wasip1::Fd) {
    create_file(dir_fd, "file");
    assert_errno!(
        wasip1::path_open(
            dir_fd,
            0,
            "file",
            wasip1::OFLAGS_CREAT | wasip1::OFLAGS_EXCL,
            0,
            0,
            0,
        )
        .expect_err("trying to create a file that already exists"),
        wasip1::ERRNO_EXIST
    );
    wasip1::path_unlink_file(dir_fd, "file").expect("removing a file");
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
    unsafe { test_path_open_create_existing(dir_fd) }
}
