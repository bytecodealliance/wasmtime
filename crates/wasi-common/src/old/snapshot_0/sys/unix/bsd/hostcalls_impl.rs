use super::osfile::OsFile;
use crate::old::snapshot_0::hostcalls_impl::PathGet;
use crate::old::snapshot_0::sys::host_impl;
use crate::old::snapshot_0::sys::unix::str_to_cstring;
use crate::old::snapshot_0::{wasi, Error, Result};
use nix::libc::{self, c_long, c_void};
use std::convert::TryInto;
use std::fs::File;
use std::os::unix::prelude::AsRawFd;

pub(crate) fn path_unlink_file(resolved: PathGet) -> Result<()> {
    use nix::errno;
    use nix::libc::unlinkat;

    let path_cstr = str_to_cstring(resolved.path())?;

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

pub(crate) fn path_symlink(old_path: &str, resolved: PathGet) -> Result<()> {
    use nix::{errno::Errno, fcntl::AtFlags, libc::symlinkat, sys::stat::fstatat};

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
        match Errno::last() {
            Errno::ENOTDIR => {
                // On BSD, symlinkat returns ENOTDIR when it should in fact
                // return a EEXIST. It seems that it gets confused with by
                // the trailing slash in the target path. Thus, we strip
                // the trailing slash and check if the path exists, and
                // adjust the error code appropriately.
                let new_path = resolved.path().trim_end_matches('/');
                if let Ok(_) = fstatat(
                    resolved.dirfd().as_raw_fd(),
                    new_path,
                    AtFlags::AT_SYMLINK_NOFOLLOW,
                ) {
                    Err(Error::EEXIST)
                } else {
                    Err(Error::ENOTDIR)
                }
            }
            x => Err(host_impl::errno_from_nix(x)),
        }
    } else {
        Ok(())
    }
}

pub(crate) fn path_rename(resolved_old: PathGet, resolved_new: PathGet) -> Result<()> {
    use nix::{errno::Errno, fcntl::AtFlags, libc::renameat, sys::stat::fstatat};
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
    cookie: wasi::__wasi_dircookie_t,
) -> Result<usize> {
    use crate::old::snapshot_0::sys::unix::bsd::osfile::DirStream;
    use libc::{fdopendir, readdir, rewinddir, seekdir, telldir};
    use nix::errno::Errno;
    use std::ffi::CStr;
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

    if cookie != wasi::__WASI_DIRCOOKIE_START {
        unsafe { seekdir(dir_stream.dir_ptr, cookie as c_long) };
    } else {
        unsafe { rewinddir(dir_stream.dir_ptr) };
    }

    let mut left = host_buf_len;
    let mut host_buf_offset: usize = 0;

    loop {
        let errno = Errno::last();
        let host_entry_ptr = unsafe { readdir(dir_stream.dir_ptr) };
        if host_entry_ptr.is_null() {
            if errno != Errno::last() {
                // TODO Is this correct?
                // According to POSIX man (for Linux though!), there was an error
                // if the errno value has changed at some point during the sequence
                // of readdir calls
                return Err(host_impl::errno_from_nix(Errno::last()));
            } else {
                // Not an error
                break;
            }
        }

        let host_entry = unsafe { *host_entry_ptr };
        let mut wasi_entry: wasi::__wasi_dirent_t = host_impl::dirent_from_host(&host_entry)?;
        // Set d_next manually:
        // * on macOS d_seekoff is not set for some reason
        // * on FreeBSD d_seekoff doesn't exist; there is d_off but it is
        //   not equivalent to the value read from telldir call
        wasi_entry.d_next = unsafe { telldir(dir_stream.dir_ptr) } as wasi::__wasi_dircookie_t;

        log::debug!("fd_readdir host_entry = {:?}", host_entry);
        log::debug!("fd_readdir wasi_entry = {:?}", wasi_entry);

        let name_len = host_entry.d_namlen.try_into()?;
        let required_space = std::mem::size_of_val(&wasi_entry) + name_len;

        if required_space > left {
            break;
        }

        let name = unsafe { CStr::from_ptr(host_entry.d_name.as_ptr()) }.to_str()?;
        log::debug!("fd_readdir entry name = {}", name);

        unsafe {
            let ptr = host_buf_ptr.offset(host_buf_offset.try_into()?) as *mut c_void
                as *mut wasi::__wasi_dirent_t;
            *ptr = wasi_entry;
        }
        host_buf_offset += std::mem::size_of_val(&wasi_entry);

        unsafe {
            std::ptr::copy_nonoverlapping(
                name.as_ptr(),
                host_buf_ptr.offset(host_buf_offset.try_into()?),
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
    advice: wasi::__wasi_advice_t,
    offset: wasi::__wasi_filesize_t,
    len: wasi::__wasi_filesize_t,
) -> Result<()> {
    use nix::errno::Errno;

    match advice {
        wasi::__WASI_ADVICE_DONTNEED => return Ok(()),
        // unfortunately, the advisory syscall in macOS doesn't take any flags of this
        // sort (unlike on Linux), hence, they are left here as a noop
        wasi::__WASI_ADVICE_SEQUENTIAL
        | wasi::__WASI_ADVICE_WILLNEED
        | wasi::__WASI_ADVICE_NOREUSE
        | wasi::__WASI_ADVICE_RANDOM
        | wasi::__WASI_ADVICE_NORMAL => {}
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
    advice: wasi::__wasi_advice_t,
    _offset: wasi::__wasi_filesize_t,
    _len: wasi::__wasi_filesize_t,
) -> Result<()> {
    match advice {
        wasi::__WASI_ADVICE_DONTNEED
        | wasi::__WASI_ADVICE_SEQUENTIAL
        | wasi::__WASI_ADVICE_WILLNEED
        | wasi::__WASI_ADVICE_NOREUSE
        | wasi::__WASI_ADVICE_RANDOM
        | wasi::__WASI_ADVICE_NORMAL => {}
        _ => return Err(Error::EINVAL),
    }

    Ok(())
}
