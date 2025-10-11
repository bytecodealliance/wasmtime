#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

unsafe fn test_stdio_isatty() {
    assert_eq!(libc::isatty(libc::STDIN_FILENO), 1, "stdin is a tty");
    assert_eq!(libc::isatty(libc::STDOUT_FILENO), 1, "stdout is a tty");
    assert_eq!(libc::isatty(libc::STDERR_FILENO), 1, "stderr is a tty");
}

fn main() {
    // Run the tests.
    unsafe { test_stdio_isatty() }
}
