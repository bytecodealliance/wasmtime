unsafe fn test_stdio_not_isatty() {
    assert_eq!(libc::isatty(libc::STDIN_FILENO), 0, "stdin is not a tty");
    assert_eq!(libc::isatty(libc::STDOUT_FILENO), 0, "stdout is not a tty");
    assert_eq!(libc::isatty(libc::STDERR_FILENO), 0, "stderr is not a tty");
}

fn main() {
    // Run the tests.
    unsafe { test_stdio_not_isatty() }
}
