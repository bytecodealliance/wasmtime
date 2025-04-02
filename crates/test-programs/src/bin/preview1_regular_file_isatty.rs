use std::{env, process};
use test_programs::preview1::open_scratch_directory;

unsafe fn test_file_isatty(dir_fd: wasip1::Fd) {
    // Create a file in the scratch directory and test if it's a tty.
    let file_fd = wasip1::path_open(dir_fd, 0, "file", wasip1::OFLAGS_CREAT, 0, 0, 0)
        .expect("opening a file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );
    assert_eq!(
        libc::isatty(file_fd as std::os::raw::c_int),
        0,
        "file is a tty"
    );
    wasip1::fd_close(file_fd).expect("closing a file");
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
    unsafe { test_file_isatty(dir_fd) }
}
