#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

use test_programs::preview1::{STDERR_FD, STDIN_FD, STDOUT_FD};

unsafe fn test_stdio() {
    for fd in &[STDIN_FD, STDOUT_FD, STDERR_FD] {
        wasip1::fd_fdstat_get(*fd).expect("fd_fdstat_get on stdio");
        assert_eq!(wasip1::fd_renumber(*fd, *fd + 100), Err(wasip1::ERRNO_BADF));
        wasip1::fd_renumber(*fd, *fd).unwrap();
    }
}

fn main() {
    // Run the tests.
    unsafe { test_stdio() }
}
