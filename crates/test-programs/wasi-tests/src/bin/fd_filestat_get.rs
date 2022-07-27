unsafe fn test_fd_filestat_get() {

    let stat = wasi::fd_filestat_get(libc::STDIN_FILENO as u32).expect("failed filestat 0");
    assert_eq!(stat.size, 0, "stdio size should be 0");
    assert_eq!(stat.atim, 0, "stdio atim should be 0");
    assert_eq!(stat.mtim, 0, "stdio mtim should be 0");
    assert_eq!(stat.ctim, 0, "stdio ctim should be 0");

    let stat = wasi::fd_filestat_get(libc::STDOUT_FILENO as u32).expect("failed filestat 1");
    assert_eq!(stat.size, 0, "stdio size should be 0");
    assert_eq!(stat.atim, 0, "stdio atim should be 0");
    assert_eq!(stat.mtim, 0, "stdio mtim should be 0");
    assert_eq!(stat.ctim, 0, "stdio ctim should be 0");

    let stat = wasi::fd_filestat_get(libc::STDERR_FILENO as u32).expect("failed filestat 2");
    assert_eq!(stat.size, 0, "stdio size should be 0");
    assert_eq!(stat.atim, 0, "stdio atim should be 0");
    assert_eq!(stat.mtim, 0, "stdio mtim should be 0");
    assert_eq!(stat.ctim, 0, "stdio ctim should be 0");
}

fn main() {
    // Run the tests.
    unsafe { test_fd_filestat_get() }
}
