use super::osfile::OsFile;
use crate::hostcalls_impl::PathGet;
use crate::sys::host_impl;
use crate::sys::unix::str_to_cstring;
use crate::{host, Error, Result};
use nix::libc::{self, c_long, c_void};
use std::convert::TryInto;
use std::fs::File;
use std::mem::MaybeUninit;
use std::os::unix::prelude::AsRawFd;

pub(crate) fn path_unlink_file(resolved: PathGet) -> Result<()> {
    use nix::errno;
    use nix::libc::unlinkat;

    let path_cstr = str_to_cstring(resolved.path())?;

    // nix doesn't expose unlinkat() yet
    let res = unsafe { unlinkat(resolved.dirfd().as_raw_fd(), path_cstr.as_ptr(), 0) };
    if res == 0 {
        Ok(())
    } else {
        Err(host_impl::errno_from_nix(errno::Errno::last()))
    }
}

pub(crate) fn path_symlink(old_path: &str, resolved: PathGet) -> Result<()> {
    use nix::{errno::Errno, libc::symlinkat};

    let old_path_cstr = str_to_cstring(old_path)?;
    let new_path_cstr = str_to_cstring(resolved.path())?;

    log::debug!("path_symlink old_path = {:?}", old_path);
    log::debug!("path_symlink resolved = {:?}", resolved);

    let res = unsafe {
        symlinkat(
            old_path_cstr.as_ptr(),
            resolved.dirfd().as_raw_fd(),
            new_path_cstr.as_ptr(),
        )
    };
    if res != 0 {
        Err(host_impl::errno_from_nix(Errno::last()))
    } else {
        Ok(())
    }
}

pub(crate) fn path_rename(resolved_old: PathGet, resolved_new: PathGet) -> Result<()> {
    use nix::libc::renameat;
    let old_path_cstr = str_to_cstring(resolved_old.path())?;
    let new_path_cstr = str_to_cstring(resolved_new.path())?;

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
    use libc::{dirent, fdopendir, readdir_r, rewinddir, seekdir};

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

    let mut entry_buf = MaybeUninit::<dirent>::uninit();
    let mut left = host_buf_len;
    let mut host_buf_offset: usize = 0;
    while left > 0 {
        let mut host_entry: *mut dirent = std::ptr::null_mut();

        // TODO
        // `readdir_r` syscall is being deprecated so we should look into
        // replacing it with `readdir` call instead.
        // Also, `readdir_r` returns a positive int on failure, and doesn't
        // set the errno.
        let res = unsafe { readdir_r(dir, entry_buf.as_mut_ptr(), &mut host_entry) };
        if res == -1 {
            return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
        }
        if host_entry.is_null() {
            break;
        }
        unsafe { entry_buf.assume_init() };
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
            std::ptr::copy_nonoverlapping(
                name_ptr as *const _,
                host_buf_ptr.offset(host_buf_offset.try_into()?) as *mut _,
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
