use more_asserts::assert_gt;

// The `wasi` crate version 0.9.0 and beyond, doesn't
// seem to define these constants, so we do it ourselves.
pub const STDIN_FD: wasi::Fd = 0x0;
pub const STDOUT_FD: wasi::Fd = 0x1;
pub const STDERR_FD: wasi::Fd = 0x2;

/// Opens a fresh file descriptor for `path` where `path` should be a preopened
/// directory.
pub fn open_scratch_directory(path: &str) -> Result<wasi::Fd, String> {
    unsafe {
        for i in 3.. {
            let stat = match wasi::fd_prestat_get(i) {
                Ok(s) => s,
                Err(_) => break,
            };
            if stat.pr_type != wasi::PREOPENTYPE_DIR {
                continue;
            }
            let mut dst = Vec::with_capacity(stat.u.dir.pr_name_len);
            if wasi::fd_prestat_dir_name(i, dst.as_mut_ptr(), dst.capacity()).is_err() {
                continue;
            }
            dst.set_len(stat.u.dir.pr_name_len);
            if dst == path.as_bytes() {
                return Ok(wasi::path_open(i, 0, ".", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
                    .expect("failed to open dir"));
            }
        }

        Err(format!("failed to find scratch dir"))
    }
}

pub unsafe fn create_file(dir_fd: wasi::Fd, filename: &str) {
    let file_fd =
        wasi::path_open(dir_fd, 0, filename, wasi::OFLAGS_CREAT, 0, 0, 0).expect("creating a file");
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );
    wasi::fd_close(file_fd).expect("closing a file");
}
