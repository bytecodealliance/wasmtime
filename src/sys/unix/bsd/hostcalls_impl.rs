use super::osfile::OsFile;
use crate::hostcalls_impl::PathGet;
use crate::sys::host_impl;
use crate::{host, Error, Result};
use nix::libc::{self, c_long, c_void};
use std::convert::TryInto;
use std::ffi::CString;
use std::fs::File;
use std::os::unix::prelude::AsRawFd;

pub(crate) fn path_unlink_file(resolved: PathGet) -> Result<()> {
    use nix::errno;
    use nix::libc::unlinkat;

    let path_cstr = CString::new(resolved.path().as_bytes()).map_err(|_| Error::EILSEQ)?;

    // nix doesn't expose unlinkat() yet
    match unsafe { unlinkat(resolved.dirfd().as_raw_fd(), path_cstr.as_ptr(), 0) } {
        0 => Ok(()),
        _ => {
            let mut e = errno::Errno::last();

            // Non-Linux implementations may return EPERM when attempting to remove a
            // directory without REMOVEDIR. While that's what POSIX specifies, it's
            // less useful. Adjust this to EISDIR. It doesn't matter that this is not
            // atomic with the unlinkat, because if the file is removed and a directory
            // is created before fstatat sees it, we're racing with that change anyway
            // and unlinkat could have legitimately seen the directory if the race had
            // turned out differently.
            use nix::fcntl::AtFlags;
            use nix::sys::stat::{fstatat, SFlag};

            if e == errno::Errno::EPERM {
                if let Ok(stat) = fstatat(
                    resolved.dirfd().as_raw_fd(),
                    resolved.path(),
                    AtFlags::AT_SYMLINK_NOFOLLOW,
                ) {
                    if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFDIR) {
                        e = errno::Errno::EISDIR;
                    }
                } else {
                    e = errno::Errno::last();
                }
            }

            Err(host_impl::errno_from_nix(e))
        }
    }
}

pub(crate) fn path_rename(resolved_old: PathGet, resolved_new: PathGet) -> Result<()> {
    use nix::{errno::Errno, fcntl::AtFlags, libc::renameat, sys::stat::fstatat};
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
        // Currently, this is verified to be correct on macOS, where
        // ENOENT can be returned in case when we try to rename a file
        // into a name with a trailing slash. On macOS, if the latter does
        // not exist, an ENOENT is thrown, whereas on Linux we observe the
        // correct behaviour of throwing an ENOTDIR since the destination is
        // indeed not a directory.
        //
        // TODO
        // Verify on other BSD-based OSes.
        match Errno::last() {
            Errno::ENOENT => {
                // check if the source path exists
                if let Ok(_) = fstatat(
                    resolved_old.dirfd().as_raw_fd(),
                    resolved_old.path(),
                    AtFlags::AT_SYMLINK_NOFOLLOW,
                ) {
                    // check if destination contains a trailing slash
                    if resolved_new.path().contains('/') {
                        Err(Error::ENOTDIR)
                    } else {
                        Err(Error::ENOENT)
                    }
                } else {
                    Err(Error::ENOENT)
                }
            }
            x => Err(host_impl::errno_from_nix(x)),
        }
    } else {
        Ok(())
    }
}

pub(crate) fn fd_readdir(
    os_file: &mut OsFile,
    host_buf: &mut [u8],
    cookie: host::__wasi_dircookie_t,
) -> Result<usize> {
    use crate::sys::unix::bsd::osfile::DirStream;
    use libc::{fdopendir, readdir, rewinddir, seekdir, telldir};
    use nix::errno::Errno;
    use std::mem::ManuallyDrop;
    use std::sync::Mutex;

    let dir_stream = match os_file.dir_stream {
        Some(ref mut dir_stream) => dir_stream,
        None => {
            let file = os_file.file.try_clone()?;
            let dir_ptr = unsafe { fdopendir(file.as_raw_fd()) };
            os_file.dir_stream.get_or_insert(Mutex::new(DirStream {
                file: ManuallyDrop::new(file),
                dir_ptr,
            }))
        }
    };
    let dir_stream = dir_stream.lock().unwrap();

    let host_buf_ptr = host_buf.as_mut_ptr();
    let host_buf_len = host_buf.len();

    if cookie != host::__WASI_DIRCOOKIE_START {
        unsafe { seekdir(dir_stream.dir_ptr, cookie as c_long) };
    } else {
        unsafe { rewinddir(dir_stream.dir_ptr) };
    }

    let mut left = host_buf_len;
    let mut host_buf_offset: usize = 0;

    loop {
        let host_entry = unsafe { readdir(dir_stream.dir_ptr) };
        if host_entry.is_null() {
            // FIXME
            // Currently, these are verified to be correct on macOS.
            // Need to still verify these on other BSD-based OSes.
            match Errno::last() {
                Errno::EBADF => return Err(Error::EBADF),
                Errno::EFAULT => return Err(Error::EFAULT),
                Errno::EIO => return Err(Error::EIO),
                _ => break, // not an error
            }
        }

        let mut entry: host::__wasi_dirent_t =
            host_impl::dirent_from_host(&unsafe { *host_entry })?;
        // Set d_next manually:
        // * on macOS d_seekoff is not set for some reason
        // * on FreeBSD d_seekoff doesn't exist; there is d_off but it is
        //   not equivalent to the value read from telldir call
        entry.d_next = unsafe { telldir(dir_stream.dir_ptr) } as host::__wasi_dircookie_t;

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

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) fn fd_advise(
    file: &File,
    advice: host::__wasi_advice_t,
    offset: host::__wasi_filesize_t,
    len: host::__wasi_filesize_t,
) -> Result<()> {
    use nix::errno::Errno;

    match advice {
        host::__WASI_ADVICE_DONTNEED => return Ok(()),
        // unfortunately, the advisory syscall in macOS doesn't take any flags of this
        // sort (unlike on Linux), hence, they are left here as a noop
        host::__WASI_ADVICE_SEQUENTIAL
        | host::__WASI_ADVICE_WILLNEED
        | host::__WASI_ADVICE_NOREUSE
        | host::__WASI_ADVICE_RANDOM
        | host::__WASI_ADVICE_NORMAL => {}
        _ => return Err(Error::EINVAL),
    }

    // From macOS man pages:
    // F_RDADVISE   Issue an advisory read async with no copy to user.
    //
    // The F_RDADVISE command operates on the following structure which holds information passed from
    // the user to the system:
    //
    // struct radvisory {
    //      off_t   ra_offset;  /* offset into the file */
    //      int     ra_count;   /* size of the read     */
    // };
    let advisory = libc::radvisory {
        ra_offset: offset.try_into()?,
        ra_count: len.try_into()?,
    };

    let res = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_RDADVISE, &advisory) };
    Errno::result(res).map(|_| ()).map_err(Error::from)
}

// TODO
// It seems that at least some BSDs do support `posix_fadvise`,
// so we should investigate further.
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(crate) fn fd_advise(
    _file: &File,
    advice: host::__wasi_advice_t,
    _offset: host::__wasi_filesize_t,
    _len: host::__wasi_filesize_t,
) -> Result<()> {
    match advice {
        host::__WASI_ADVICE_DONTNEED
        | host::__WASI_ADVICE_SEQUENTIAL
        | host::__WASI_ADVICE_WILLNEED
        | host::__WASI_ADVICE_NOREUSE
        | host::__WASI_ADVICE_RANDOM
        | host::__WASI_ADVICE_NORMAL => {}
        _ => return Err(Error::EINVAL),
    }

    Ok(())
}
