use std::{env, process};
use wasi_tests::{create_file, open_scratch_directory};

unsafe fn test_readlink(dir_fd: wasi::Fd) {
    // Create a file in the scratch directory.
    create_file(dir_fd, "target");

    // Create a symlink
    wasi::path_symlink("target", dir_fd, "symlink").expect("creating a symlink");

    // Read link into the buffer
    let buf = &mut [0u8; 10];
    let bufused = wasi::path_readlink(dir_fd, "symlink", buf.as_mut_ptr(), buf.len())
        .expect("readlink should succeed");
    assert_eq!(bufused, 6, "should use 6 bytes of the buffer");
    assert_eq!(&buf[..6], b"target", "buffer should contain 'target'");
    assert_eq!(
        &buf[6..],
        &[0u8; 4],
        "the remaining bytes should be untouched"
    );

    // Read link into smaller buffer than the actual link's length
    let buf = &mut [0u8; 4];
    let err = wasi::path_readlink(dir_fd, "symlink", buf.as_mut_ptr(), buf.len())
        .err()
        .expect("readlink with too-small buffer should fail");
    assert_eq!(
        err.raw_error(),
        wasi::ERRNO_RANGE,
        "readlink with too-small buffer should give ERANGE"
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
    unsafe { test_readlink(dir_fd) }
}
