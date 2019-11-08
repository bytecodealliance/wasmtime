pub mod utils;
pub mod wasi_wrappers;

use libc;
use std::ffi::CString;
use std::io;
use wasi::wasi_unstable;

pub fn open_scratch_directory(path: &str) -> Result<wasi_unstable::Fd, String> {
    // Open the scratch directory.
    let dir_fd: wasi_unstable::Fd = unsafe {
        let cstr = CString::new(path.as_bytes()).unwrap();
        libc::open(cstr.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY)
    } as wasi_unstable::Fd;

    if (dir_fd as std::os::raw::c_int) < 0 {
        Err(format!(
            "error opening scratch directory '{}': {}",
            path,
            io::Error::last_os_error()
        ))
    } else {
        Ok(dir_fd)
    }
}
