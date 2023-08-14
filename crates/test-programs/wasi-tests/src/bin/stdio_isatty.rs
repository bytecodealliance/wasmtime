unsafe fn test_stdio_isatty() {
    assert_eq!(
        libc::isatty(libc::STDIN_FILENO as std::os::raw::c_int),
        1,
        "stdin is a tty"
    );
    assert_eq!(
        libc::isatty(libc::STDOUT_FILENO as std::os::raw::c_int),
        1,
        "stdout is a tty"
    );
    assert_eq!(
        libc::isatty(libc::STDERR_FILENO as std::os::raw::c_int),
        1,
        "stderr is a tty"
    );
}

fn main() {
    // Run the tests.
    unsafe { test_stdio_isatty() }
}
