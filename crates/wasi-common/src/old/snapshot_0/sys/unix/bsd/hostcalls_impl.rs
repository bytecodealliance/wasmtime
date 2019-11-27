use super::super::dir::{Dir, Entry, SeekLoc};
use super::oshandle::OsHandle;
use crate::old::snapshot_0::hostcalls_impl::{Dirent, PathGet};
use crate::old::snapshot_0::sys::host_impl;
use crate::old::snapshot_0::sys::unix::str_to_cstring;
use crate::old::snapshot_0::{wasi, Error, Result};
use nix::libc;
use std::convert::TryInto;
use std::fs::File;
use std::os::unix::prelude::AsRawFd;
use std::sync::MutexGuard;

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

pub(crate) fn fd_readdir<'a>(
    os_handle: &'a mut OsHandle,
    cookie: wasi::__wasi_dircookie_t,
) -> Result<impl Iterator<Item = Result<Dirent>> + 'a> {
    use std::sync::Mutex;

    let dir = match os_handle.dir {
        Some(ref mut dir) => dir,
        None => {
            // We need to duplicate the fd, because `opendir(3)`:
            //     Upon successful return from fdopendir(), the file descriptor is under
            //     control of the system, and if any attempt is made to close the file
            //     descriptor, or to modify the state of the associated description other
            //     than by means of closedir(), readdir(), readdir_r(), or rewinddir(),
            //     the behaviour is undefined.
            let fd = (*os_handle).try_clone()?;
            let dir = Dir::from(fd)?;
            os_handle.dir.get_or_insert(Mutex::new(dir))
        }
    };
    let mut dir = dir.lock().unwrap();

    // Seek if needed. Unless cookie is wasi::__WASI_DIRCOOKIE_START,
    // new items may not be returned to the caller.
    if cookie == wasi::__WASI_DIRCOOKIE_START {
        log::trace!("     | fd_readdir: doing rewinddir");
        dir.rewind();
    } else {
        log::trace!("     | fd_readdir: doing seekdir to {}", cookie);
        let loc = unsafe { SeekLoc::from_raw(cookie as i64) };
        dir.seek(loc);
    }

    Ok(DirIter(dir).map(|entry| {
        let (entry, loc): (Entry, SeekLoc) = entry?;
        Ok(Dirent {
            name: entry
                // TODO can we reuse path_from_host for CStr?
                .file_name()
                .to_str()?
                .to_owned(),
            ino: entry.ino(),
            ftype: entry.file_type().into(),
            // Set cookie manually:
            // * on macOS d_seekoff is not set for some reason
            // * on FreeBSD d_seekoff doesn't exist; there is d_off but it is
            //   not equivalent to the value read from telldir call
            cookie: loc.to_raw().try_into()?,
        })
    }))
}

struct DirIter<'a>(MutexGuard<'a, Dir>);

impl<'a> Iterator for DirIter<'a> {
    type Item = nix::Result<(Entry, SeekLoc)>;

    fn next(&mut self) -> Option<Self::Item> {
        use libc::readdir;
        use nix::{errno::Errno, Error};

        unsafe {
            let errno = Errno::last();
            let ent = readdir((self.0).0.as_ptr());
            if ent.is_null() {
                if errno != Errno::last() {
                    // TODO This should be verified on different BSD-flavours.
                    //
                    // According to 4.3BSD/POSIX.1-2001 man pages, there was an error
                    // if the errno value has changed at some point during the sequence
                    // of readdir calls.
                    Some(Err(Error::last()))
                } else {
                    // Not an error. We've simply reached the end of the stream.
                    None
                }
            } else {
                let entry = Entry(*ent);
                let loc = self.0.tell();
                Some(Ok((entry, loc)))
            }
        }
    }
}
