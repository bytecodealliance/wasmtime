use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_file, create_file};
use wasi_tests::wasi_wrappers::{wasi_path_readlink, wasi_path_symlink};

unsafe fn test_readlink(dir_fd: wasi_unstable::Fd) {
    // Create a file in the scratch directory.
    create_file(dir_fd, "target");

    // Create a symlink
    assert!(
        wasi_path_symlink("target", dir_fd, "symlink").is_ok(),
        "creating a symlink"
    );

    // Read link into the buffer
    let buf = &mut [0u8; 10];
    let mut bufused: usize = 0;
    let mut status = wasi_path_readlink(dir_fd, "symlink", buf, &mut bufused);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "readlink should succeed"
    );
    assert_eq!(bufused, 6, "should use 6 bytes of the buffer");
    assert_eq!(&buf[..6], b"target", "buffer should contain 'target'");
    assert_eq!(
        &buf[6..],
        &[0u8; 4],
        "the remaining bytes should be untouched"
    );

    // Read link into smaller buffer than the actual link's length
    let buf = &mut [0u8; 4];
    let mut bufused: usize = 0;
    status = wasi_path_readlink(dir_fd, "symlink", buf, &mut bufused);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "readlink should succeed"
    );
    assert_eq!(bufused, 4, "should use all 4 bytes of the buffer");
    assert_eq!(buf, b"targ", "buffer should contain 'targ'");

    // Clean up.
    cleanup_file(dir_fd, "target");
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
    unsafe { test_readlink(dir_fd) }
}
