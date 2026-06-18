#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

use std::env;
use std::process;
use test_programs::preview1::open_scratch_directory;

const RW_ALIAS_FILENAME: &str = "alias.txt";
const RO_TEST_FILENAME: &str = "test.txt";
const RO_EXPECTED_CONTENTS: &[u8] = b"read only test file\n";

unsafe fn test_ro_file_has_expected_contents(dir_fd: wasip1::Fd) {
    // Open a file for reading
    let file_fd = wasip1::path_open(dir_fd, 0, RO_TEST_FILENAME, 0, wasip1::RIGHTS_FD_READ, 0, 0)
        .expect("opening test.txt for reading");

    // Read the file's contents
    let buffer = &mut [0u8; 100];
    let nread = wasip1::fd_read(
        file_fd,
        &[wasip1::Iovec {
            buf: buffer.as_mut_ptr(),
            buf_len: buffer.len(),
        }],
    )
    .expect("reading file content");

    // The file should be as created by the test harness
    assert_eq!(nread, RO_EXPECTED_CONTENTS.len(), "expected untouched file");
    assert_eq!(
        &buffer[..nread],
        RO_EXPECTED_CONTENTS,
        "expected untouched file contents"
    );

    wasip1::fd_close(file_fd).expect("closing the file");
}

unsafe fn test_file_rename_across_perms(rw_dir_fd: wasip1::Fd, ro_dir_fd: wasip1::Fd) {
    // Check test preconditions.
    test_ro_file_has_expected_contents(ro_dir_fd);

    // Create a hardlink inside the file ro dir so there are two files pointing to
    // the read-only file.
    match wasip1::path_link(ro_dir_fd, 0, RO_TEST_FILENAME, ro_dir_fd, RW_ALIAS_FILENAME) {
        // The readonly dir isnt recreated fresh per test mode in the p2
        // runner, so just allow this to fail with exists because its very
        // tedious to restructure everything to fix this properly
        Ok(()) | Err(wasip1::ERRNO_EXIST) => {}
        _ => panic!("should be possible to create link inside ro file domain"),
    }

    // Renaming that file into the file rw dir should fail with permissions
    // error, otherwise it would permit opening the ro file as rw
    let err = wasip1::path_rename(ro_dir_fd, RW_ALIAS_FILENAME, rw_dir_fd, RW_ALIAS_FILENAME);
    assert!(
        err.is_err(),
        "path_rename should fail because link source readonly, dest is readwrite"
    );
    assert_eq!(
        err.err().unwrap(),
        wasip1::ERRNO_PERM,
        "path_rename should fail with ERRNO_PERM"
    );

    // Check that contents of link dest did not change
    test_ro_file_has_expected_contents(ro_dir_fd);
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

    // Open read-write scratch directory
    let rw_dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("failed to open scratch directory: {err}");
            process::exit(1)
        }
    };

    // This test program requires a special preopen at the path "readonly",
    // which the host enforces as read-only. Unlike other test programs, this
    // directory's path not passed in as an argument, because modifications to
    // the testing harness would be too invasive.
    let ro_dir_fd = match open_scratch_directory("readonly") {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("failed to open readonly preopen: {err}");
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe {
        test_file_rename_across_perms(rw_dir_fd, ro_dir_fd);
    }
}
