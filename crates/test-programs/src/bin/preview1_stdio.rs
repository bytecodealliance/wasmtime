use test_programs::preview1::{STDERR_FD, STDIN_FD, STDOUT_FD};

unsafe fn test_stdio() {
    for fd in &[STDIN_FD, STDOUT_FD, STDERR_FD] {
        wasip1::fd_fdstat_get(*fd).expect("fd_fdstat_get on stdio");
        wasip1::fd_renumber(*fd, *fd + 100).expect("renumbering stdio");
    }
}

fn main() {
    // Run the tests.
    unsafe { test_stdio() }
}
