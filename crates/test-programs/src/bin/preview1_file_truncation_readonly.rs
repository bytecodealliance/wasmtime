use std::process;
use test_programs::preview1::open_scratch_directory;

const FILENAME: &str = "test.txt";
unsafe fn test_file_has_expected_contents(dir_fd: wasi::Fd) {
    // Open a file for reading
    let file_fd = wasi::path_open(dir_fd, 0, FILENAME, 0, wasi::RIGHTS_FD_READ, 0, 0)
        .expect("opening test.txt for reading");

    // Read the file's contents
    let buffer = &mut [0u8; 100];
    let nread = wasi::fd_read(
        file_fd,
        &[wasi::Iovec {
            buf: buffer.as_mut_ptr(),
            buf_len: buffer.len(),
        }],
    )
    .expect("reading file content");

    const EXPECTED_CONTENTS: &[u8] = b"truncation test file\n";
    // The file should be as created by the test harness, not truncated.
    assert_eq!(nread, EXPECTED_CONTENTS.len(), "expected untouched file");
    assert_eq!(
        &buffer[..nread],
        EXPECTED_CONTENTS,
        "expected untouched file contents"
    );

    wasi::fd_close(file_fd).expect("closing the file");
}

unsafe fn test_file_truncation_readonly(dir_fd: wasi::Fd) {
    // Check test preconditions.
    test_file_has_expected_contents(dir_fd);

    // Opening the file for truncation should fail.
    let err = wasi::path_open(
        dir_fd,
        0,
        FILENAME,
        wasi::OFLAGS_TRUNC,
        wasi::RIGHTS_FD_READ,
        0,
        0,
    );
    assert!(err.is_err(), "opening file for truncation should fail");
    assert_eq!(
        err.err().unwrap(),
        wasi::ERRNO_PERM,
        "opening file for truncation should fail with PERM"
    );

    // Check that truncation did not occur.
    test_file_has_expected_contents(dir_fd);
}

fn main() {
    // This test program requires a special preopen at the path "readonly",
    // which the host enforces as read-only. Unlike other test programs, this
    // directory's path not passed in as an argument, because modifications to
    // the testing harness would be too invasive.
    let dir_fd = match open_scratch_directory("readonly") {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe {
        test_file_truncation_readonly(dir_fd);
    }
}
