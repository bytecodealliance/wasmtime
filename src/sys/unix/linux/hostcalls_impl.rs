use super::osfile::OsFile;
use crate::hostcalls_impl::PathGet;
use crate::sys::host_impl;
use crate::{host, Error, Result};
use nix::libc::{self, c_long, c_void};
use std::convert::TryInto;
use std::ffi::CString;
use std::fs::File;
use std::os::unix::prelude::AsRawFd;

pub(crate) fn path_rename(resolved_old: PathGet, resolved_new: PathGet) -> Result<()> {
    use nix::libc::renameat;
    let old_path_cstr = CString::new(resolved_old.path().as_bytes()).map_err(|_| Error::EILSEQ)?;
    let new_path_cstr = CString::new(resolved_new.path().as_bytes()).map_err(|_| Error::EILSEQ)?;

    let res = unsafe {
        renameat(
            resolved_old.dirfd().as_raw_fd(),
            old_path_cstr.as_ptr(),
            resolved_new.dirfd().as_raw_fd(),
            new_path_cstr.as_ptr(),
        )
    };
    if res != 0 {
        Err(host_impl::errno_from_nix(nix::errno::Errno::last()))
    } else {
        Ok(())
    }
}

pub(crate) fn fd_readdir(
    os_file: &mut OsFile,
    host_buf: &mut [u8],
    cookie: host::__wasi_dircookie_t,
) -> Result<usize> {
    use libc::{dirent, fdopendir, memcpy, readdir_r, rewinddir, seekdir};

    let host_buf_ptr = host_buf.as_mut_ptr();
    let host_buf_len = host_buf.len();
    let dir = unsafe { fdopendir(os_file.as_raw_fd()) };
    if dir.is_null() {
        return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
    }

    if cookie != host::__WASI_DIRCOOKIE_START {
        unsafe { seekdir(dir, cookie as c_long) };
    } else {
        // If cookie set to __WASI_DIRCOOKIE_START, rewind the dir ptr
        // to the start of the stream.
        unsafe { rewinddir(dir) };
    }

    let mut entry_buf = unsafe { std::mem::uninitialized::<dirent>() };
    let mut left = host_buf_len;
    let mut host_buf_offset: usize = 0;
    while left > 0 {
        let mut host_entry: *mut dirent = std::ptr::null_mut();

        // TODO
        // `readdir_r` syscall is being deprecated so we should look into
        // replacing it with `readdir` call instead.
        // Also, `readdir_r` returns a positive int on failure, and doesn't
        // set the errno.
        let res = unsafe { readdir_r(dir, &mut entry_buf, &mut host_entry) };
        if res == -1 {
            return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
        }
        if host_entry.is_null() {
            break;
        }
        let entry: host::__wasi_dirent_t = host_impl::dirent_from_host(&unsafe { *host_entry })?;

        log::debug!("fd_readdir entry = {:?}", entry);

        let name_len = entry.d_namlen.try_into()?;
        let required_space = std::mem::size_of_val(&entry) + name_len;
        if required_space > left {
            break;
        }
        unsafe {
            let ptr = host_buf_ptr.offset(host_buf_offset.try_into()?) as *mut c_void
                as *mut host::__wasi_dirent_t;
            *ptr = entry;
        }
        host_buf_offset += std::mem::size_of_val(&entry);
        let name_ptr = unsafe { *host_entry }.d_name.as_ptr();
        unsafe {
            memcpy(
                host_buf_ptr.offset(host_buf_offset.try_into()?) as *mut _,
                name_ptr as *const _,
                name_len,
            )
        };
        host_buf_offset += name_len;
        left -= required_space;
    }

    Ok(host_buf_len - left)
}

pub(crate) fn fd_advise(
    file: &File,
    advice: host::__wasi_advice_t,
    offset: host::__wasi_filesize_t,
    len: host::__wasi_filesize_t,
) -> Result<()> {
    {
        use nix::fcntl::{posix_fadvise, PosixFadviseAdvice};

        let offset = offset.try_into()?;
        let len = len.try_into()?;
        let host_advice = match advice {
            host::__WASI_ADVICE_DONTNEED => PosixFadviseAdvice::POSIX_FADV_DONTNEED,
            host::__WASI_ADVICE_SEQUENTIAL => PosixFadviseAdvice::POSIX_FADV_SEQUENTIAL,
            host::__WASI_ADVICE_WILLNEED => PosixFadviseAdvice::POSIX_FADV_WILLNEED,
            host::__WASI_ADVICE_NOREUSE => PosixFadviseAdvice::POSIX_FADV_NOREUSE,
            host::__WASI_ADVICE_RANDOM => PosixFadviseAdvice::POSIX_FADV_RANDOM,
            host::__WASI_ADVICE_NORMAL => PosixFadviseAdvice::POSIX_FADV_NORMAL,
            _ => return Err(Error::EINVAL),
        };

        posix_fadvise(file.as_raw_fd(), offset, len, host_advice)?;
    }

    Ok(())
}
