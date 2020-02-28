use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn test_readlink_no_buffer(dir_fd: wasi::Fd) {
    // First create a dangling symlink.
    wasi::path_symlink("target", dir_fd, "symlink").expect("creating a symlink");

    // Readlink it into a non-existent buffer.
    let bufused = wasi::path_readlink(dir_fd, "symlink", (&mut []).as_mut_ptr(), 0)
        .expect("readlink with a 0-sized buffer should succeed");
    assert_eq!(
        bufused, 0,
        "readlink with a 0-sized buffer should return 'bufused' 0"
    );

    // Clean up.
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
    unsafe { test_readlink_no_buffer(dir_fd) }
}
