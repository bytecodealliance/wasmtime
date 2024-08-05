use std::{env, process};
use test_programs::preview1::{create_file, open_scratch_directory};

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
    let bufused = wasi::path_readlink(dir_fd, "symlink", buf.as_mut_ptr(), buf.len())
        .expect("readlink with too-small buffer should silently truncate");
    assert_eq!(bufused, 4);
    assert_eq!(buf, b"targ");

    // Clean up.
    wasi::path_unlink_file(dir_fd, "target").expect("removing a file");
    wasi::path_unlink_file(dir_fd, "symlink").expect("removing a file");
}

unsafe fn test_incremental_readlink(dir_fd: wasi::Fd) {
    let filename = "Действие";
    create_file(dir_fd, filename);

    wasi::path_symlink(filename, dir_fd, "symlink").expect("creating a symlink");

    let mut buf = Vec::new();
    loop {
        if buf.capacity() > 2 * filename.len() {
            panic!()
        }
        let bufused = wasi::path_readlink(dir_fd, "symlink", buf.as_mut_ptr(), buf.capacity())
            .expect("readlink should succeed");
        buf.set_len(bufused);
        if buf.capacity() > filename.len() {
            assert!(buf.starts_with(filename.as_bytes()));
            break;
        }
        buf = Vec::with_capacity(buf.capacity() + 1);
    }
    wasi::path_unlink_file(dir_fd, filename).expect("removing a file");
    wasi::path_unlink_file(dir_fd, "symlink").expect("removing a file");
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
    unsafe { test_readlink(dir_fd) }
    unsafe { test_incremental_readlink(dir_fd) }
}
