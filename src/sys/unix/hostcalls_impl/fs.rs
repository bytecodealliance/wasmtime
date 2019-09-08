#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use super::fs_helpers::*;
use crate::helpers::systemtime_to_timestamp;
use crate::hostcalls_impl::PathGet;
use crate::sys::host_impl::{self, errno_from_nix};
use crate::{host, Error, Result};
use nix::libc::{self, c_long, c_void};
use std::convert::TryInto;
use std::ffi::CString;
use std::fs::{File, Metadata};
use std::os::unix::fs::FileExt;
use std::os::unix::prelude::{AsRawFd, FromRawFd};

pub(crate) fn fd_pread(
    file: &File,
    buf: &mut [u8],
    offset: host::__wasi_filesize_t,
) -> Result<usize> {
    file.read_at(buf, offset).map_err(Into::into)
}

pub(crate) fn fd_pwrite(file: &File, buf: &[u8], offset: host::__wasi_filesize_t) -> Result<usize> {
    file.write_at(buf, offset).map_err(Into::into)
}

pub(crate) fn fd_fdstat_get(fd: &File) -> Result<host::__wasi_fdflags_t> {
    use nix::fcntl::{fcntl, OFlag, F_GETFL};
    match fcntl(fd.as_raw_fd(), F_GETFL).map(OFlag::from_bits_truncate) {
        Ok(flags) => Ok(host_impl::fdflags_from_nix(flags)),
        Err(e) => Err(host_impl::errno_from_nix(e.as_errno().unwrap())),
    }
}

pub(crate) fn fd_fdstat_set_flags(fd: &File, fdflags: host::__wasi_fdflags_t) -> Result<()> {
    use nix::fcntl::{fcntl, F_SETFL};
    let nix_flags = host_impl::nix_from_fdflags(fdflags);
    match fcntl(fd.as_raw_fd(), F_SETFL(nix_flags)) {
        Ok(_) => Ok(()),
        Err(e) => Err(host_impl::errno_from_nix(e.as_errno().unwrap())),
    }
}

#[cfg(target_os = "linux")]
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
            host::__WASI_ADVICE_WILLNEED => PosixFadviseAdvice::POSIX_FADV_DONTNEED,
            host::__WASI_ADVICE_NOREUSE => PosixFadviseAdvice::POSIX_FADV_NOREUSE,
            host::__WASI_ADVICE_RANDOM => PosixFadviseAdvice::POSIX_FADV_RANDOM,
            host::__WASI_ADVICE_NORMAL => PosixFadviseAdvice::POSIX_FADV_NORMAL,
            _ => return Err(Error::EINVAL),
        };

        posix_fadvise(file.as_raw_fd(), offset, len, host_advice)
            .map_err(|err| errno_from_nix(err.as_errno().unwrap()))?;
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
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

pub(crate) fn path_create_directory(resolved: PathGet) -> Result<()> {
    use nix::libc::mkdirat;
    let path_cstr = CString::new(resolved.path().as_bytes()).map_err(|_| Error::EILSEQ)?;
    // nix doesn't expose mkdirat() yet
    match unsafe { mkdirat(resolved.dirfd().as_raw_fd(), path_cstr.as_ptr(), 0o777) } {
        0 => Ok(()),
        _ => Err(host_impl::errno_from_nix(nix::errno::Errno::last())),
    }
}

pub(crate) fn path_link(resolved_old: PathGet, resolved_new: PathGet) -> Result<()> {
    use nix::libc::linkat;
    let old_path_cstr = CString::new(resolved_old.path().as_bytes()).map_err(|_| Error::EILSEQ)?;
    let new_path_cstr = CString::new(resolved_new.path().as_bytes()).map_err(|_| Error::EILSEQ)?;

    // Not setting AT_SYMLINK_FOLLOW fails on most filesystems
    let atflags = libc::AT_SYMLINK_FOLLOW;
    let res = unsafe {
        linkat(
            resolved_old.dirfd().as_raw_fd(),
            old_path_cstr.as_ptr(),
            resolved_new.dirfd().as_raw_fd(),
            new_path_cstr.as_ptr(),
            atflags,
        )
    };
    if res != 0 {
        Err(host_impl::errno_from_nix(nix::errno::Errno::last()))
    } else {
        Ok(())
    }
}

pub(crate) fn path_open(
    resolved: PathGet,
    read: bool,
    write: bool,
    oflags: host::__wasi_oflags_t,
    fs_flags: host::__wasi_fdflags_t,
) -> Result<File> {
    use nix::errno::Errno;
    use nix::fcntl::{openat, AtFlags, OFlag};
    use nix::sys::stat::{fstatat, Mode, SFlag};

    let mut nix_all_oflags = if read && write {
        OFlag::O_RDWR
    } else if write {
        OFlag::O_WRONLY
    } else {
        OFlag::O_RDONLY
    };

    // on non-Capsicum systems, we always want nofollow
    nix_all_oflags.insert(OFlag::O_NOFOLLOW);

    // convert open flags
    nix_all_oflags.insert(host_impl::nix_from_oflags(oflags));

    // convert file descriptor flags
    nix_all_oflags.insert(host_impl::nix_from_fdflags(fs_flags));

    // Call openat. Use mode 0o666 so that we follow whatever the user's
    // umask is, but don't set the executable flag, because it isn't yet
    // meaningful for WASI programs to create executable files.
    let new_fd = match openat(
        resolved.dirfd().as_raw_fd(),
        resolved.path(),
        nix_all_oflags,
        Mode::from_bits_truncate(0o666),
    ) {
        Ok(fd) => fd,
        Err(e) => {
            match e.as_errno() {
                // Linux returns ENXIO instead of EOPNOTSUPP when opening a socket
                Some(Errno::ENXIO) => {
                    if let Ok(stat) = fstatat(
                        resolved.dirfd().as_raw_fd(),
                        resolved.path(),
                        AtFlags::AT_SYMLINK_NOFOLLOW,
                    ) {
                        if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFSOCK) {
                            return Err(Error::ENOTSUP);
                        } else {
                            return Err(Error::ENXIO);
                        }
                    } else {
                        return Err(Error::ENXIO);
                    }
                }
                // Linux returns ENOTDIR instead of ELOOP when using O_NOFOLLOW|O_DIRECTORY
                // on a symlink.
                Some(Errno::ENOTDIR)
                    if !(nix_all_oflags & (OFlag::O_NOFOLLOW | OFlag::O_DIRECTORY)).is_empty() =>
                {
                    if let Ok(stat) = fstatat(
                        resolved.dirfd().as_raw_fd(),
                        resolved.path(),
                        AtFlags::AT_SYMLINK_NOFOLLOW,
                    ) {
                        if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFLNK) {
                            return Err(Error::ELOOP);
                        }
                    }
                    return Err(Error::ENOTDIR);
                }
                // FreeBSD returns EMLINK instead of ELOOP when using O_NOFOLLOW on
                // a symlink.
                Some(Errno::EMLINK) if !(nix_all_oflags & OFlag::O_NOFOLLOW).is_empty() => {
                    return Err(Error::ELOOP);
                }
                Some(e) => return Err(host_impl::errno_from_nix(e)),
                None => return Err(Error::ENOSYS),
            }
        }
    };

    // Determine the type of the new file descriptor and which rights contradict with this type
    Ok(unsafe { File::from_raw_fd(new_fd) })
}

pub(crate) fn fd_readdir(
    fd: &File,
    host_buf: &mut [u8],
    cookie: host::__wasi_dircookie_t,
) -> Result<usize> {
    use libc::{dirent, fdopendir, memcpy, readdir_r, rewinddir, seekdir};

    let host_buf_ptr = host_buf.as_mut_ptr();
    let host_buf_len = host_buf.len();
    let dir = unsafe { fdopendir(fd.as_raw_fd()) };
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
        let res = unsafe { readdir_r(dir, &mut entry_buf, &mut host_entry) };
        if res == -1 {
            return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
        }
        if host_entry.is_null() {
            break;
        }
        let entry: host::__wasi_dirent_t = host_impl::dirent_from_host(&unsafe { *host_entry })?;
        let name_len = entry.d_namlen as usize;
        let required_space = std::mem::size_of_val(&entry) + name_len;
        if required_space > left {
            break;
        }
        unsafe {
            let ptr = host_buf_ptr.offset(host_buf_offset as isize) as *mut c_void
                as *mut host::__wasi_dirent_t;
            *ptr = entry;
        }
        host_buf_offset += std::mem::size_of_val(&entry);
        let name_ptr = unsafe { *host_entry }.d_name.as_ptr();
        unsafe {
            memcpy(
                host_buf_ptr.offset(host_buf_offset as isize) as *mut _,
                name_ptr as *const _,
                name_len,
            )
        };
        host_buf_offset += name_len;
        left -= required_space;
    }
    Ok(host_buf_len - left)
}

pub(crate) fn path_readlink(resolved: PathGet, buf: &mut [u8]) -> Result<usize> {
    use nix::errno::Errno;
    let path_cstr = CString::new(resolved.path().as_bytes()).map_err(|_| Error::EILSEQ)?;

    // Linux requires that the buffer size is positive, whereas POSIX does not.
    // Use a fake buffer to store the results if the size is zero.
    // TODO: instead of using raw libc::readlinkat call here, this should really
    // be fixed in `nix` crate
    let fakebuf: &mut [u8] = &mut [0];
    let buf_len = buf.len();
    let len = unsafe {
        libc::readlinkat(
            resolved.dirfd().as_raw_fd(),
            path_cstr.as_ptr() as *const libc::c_char,
            if buf_len == 0 {
                fakebuf.as_mut_ptr()
            } else {
                buf.as_mut_ptr()
            } as *mut libc::c_char,
            if buf_len == 0 { fakebuf.len() } else { buf_len },
        )
    };

    if len < 0 {
        Err(host_impl::errno_from_nix(Errno::last()))
    } else {
        let len = len as usize;
        Ok(if len < buf_len { len } else { buf_len })
    }
}

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

pub(crate) fn fd_filestat_get_impl(file: &std::fs::File) -> Result<host::__wasi_filestat_t> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata()?;
    Ok(host::__wasi_filestat_t {
        st_dev: metadata.dev(),
        st_ino: metadata.ino(),
        st_nlink: metadata.nlink().try_into()?, // u64 doesn't fit into u32
        st_size: metadata.len(),
        st_atim: systemtime_to_timestamp(metadata.accessed()?)?,
        st_ctim: metadata.ctime().try_into()?, // i64 doesn't fit into u64
        st_mtim: systemtime_to_timestamp(metadata.modified()?)?,
        st_filetype: filetype(file, &metadata)?,
    })
}

fn filetype(file: &File, metadata: &Metadata) -> Result<host::__wasi_filetype_t> {
    use nix::sys::socket::{self, SockType};
    use std::os::unix::fs::FileTypeExt;
    let ftype = metadata.file_type();
    if ftype.is_file() {
        Ok(host::__WASI_FILETYPE_REGULAR_FILE)
    } else if ftype.is_dir() {
        Ok(host::__WASI_FILETYPE_DIRECTORY)
    } else if ftype.is_symlink() {
        Ok(host::__WASI_FILETYPE_SYMBOLIC_LINK)
    } else if ftype.is_char_device() {
        Ok(host::__WASI_FILETYPE_CHARACTER_DEVICE)
    } else if ftype.is_block_device() {
        Ok(host::__WASI_FILETYPE_BLOCK_DEVICE)
    } else if ftype.is_socket() {
        match socket::getsockopt(file.as_raw_fd(), socket::sockopt::SockType)
            .map_err(|err| err.as_errno().unwrap())
            .map_err(host_impl::errno_from_nix)?
        {
            SockType::Datagram => Ok(host::__WASI_FILETYPE_SOCKET_DGRAM),
            SockType::Stream => Ok(host::__WASI_FILETYPE_SOCKET_STREAM),
            _ => Ok(host::__WASI_FILETYPE_UNKNOWN),
        }
    } else {
        Ok(host::__WASI_FILETYPE_UNKNOWN)
    }
}

pub(crate) fn path_filestat_get(
    resolved: PathGet,
    dirflags: host::__wasi_lookupflags_t,
) -> Result<host::__wasi_filestat_t> {
    use nix::fcntl::AtFlags;
    use nix::sys::stat::fstatat;

    let atflags = match dirflags {
        0 => AtFlags::empty(),
        _ => AtFlags::AT_SYMLINK_NOFOLLOW,
    };

    let filestat = fstatat(resolved.dirfd().as_raw_fd(), resolved.path(), atflags)
        .map_err(|err| host_impl::errno_from_nix(err.as_errno().unwrap()))?;
    host_impl::filestat_from_nix(filestat)
}

pub(crate) fn path_filestat_set_times(
    resolved: PathGet,
    dirflags: host::__wasi_lookupflags_t,
    st_atim: host::__wasi_timestamp_t,
    st_mtim: host::__wasi_timestamp_t,
    fst_flags: host::__wasi_fstflags_t,
) -> Result<()> {
    use nix::sys::stat::{utimensat, UtimensatFlags};
    use nix::sys::time::{TimeSpec, TimeValLike};

    // FIXME this should be a part of nix
    fn timespec_omit() -> TimeSpec {
        let raw_ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: utime_omit(),
        };
        unsafe { std::mem::transmute(raw_ts) }
    };

    fn timespec_now() -> TimeSpec {
        let raw_ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: utime_now(),
        };
        unsafe { std::mem::transmute(raw_ts) }
    };

    let set_atim = fst_flags & host::__WASI_FILESTAT_SET_ATIM != 0;
    let set_atim_now = fst_flags & host::__WASI_FILESTAT_SET_ATIM_NOW != 0;
    let set_mtim = fst_flags & host::__WASI_FILESTAT_SET_MTIM != 0;
    let set_mtim_now = fst_flags & host::__WASI_FILESTAT_SET_MTIM_NOW != 0;

    if (set_atim && set_atim_now) || (set_mtim && set_mtim_now) {
        return Err(Error::EINVAL);
    }

    let atflags = match dirflags {
        host::__WASI_LOOKUP_SYMLINK_FOLLOW => UtimensatFlags::FollowSymlink,
        _ => UtimensatFlags::NoFollowSymlink,
    };

    let atim = if set_atim_now {
        let st_atim = st_atim.try_into()?;
        TimeSpec::nanoseconds(st_atim)
    } else if set_atim_now {
        timespec_now()
    } else {
        timespec_omit()
    };

    let mtim = if set_mtim {
        let st_mtim = st_mtim.try_into()?;
        TimeSpec::nanoseconds(st_mtim)
    } else if set_atim_now {
        timespec_now()
    } else {
        timespec_omit()
    };

    let fd = resolved.dirfd().as_raw_fd().into();
    utimensat(fd, resolved.path(), &atim, &mtim, atflags).map_err(Into::into)
}

pub(crate) fn path_symlink(old_path: &str, resolved: PathGet) -> Result<()> {
    use nix::libc::symlinkat;

    let old_path_cstr = CString::new(old_path.as_bytes()).map_err(|_| Error::EILSEQ)?;
    let new_path_cstr = CString::new(resolved.path().as_bytes()).map_err(|_| Error::EILSEQ)?;

    let res = unsafe {
        symlinkat(
            old_path_cstr.as_ptr(),
            resolved.dirfd().as_raw_fd(),
            new_path_cstr.as_ptr(),
        )
    };
    if res != 0 {
        Err(host_impl::errno_from_nix(nix::errno::Errno::last()))
    } else {
        Ok(())
    }
}

pub(crate) fn path_unlink_file(resolved: PathGet) -> Result<()> {
    use nix::errno;
    use nix::libc::unlinkat;

    let path_cstr = CString::new(resolved.path().as_bytes()).map_err(|_| Error::EILSEQ)?;

    // nix doesn't expose unlinkat() yet
    match unsafe { unlinkat(resolved.dirfd().as_raw_fd(), path_cstr.as_ptr(), 0) } {
        0 => Ok(()),
        _ => {
            let mut e = errno::Errno::last();

            #[cfg(not(linux))]
            {
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
            }

            Err(host_impl::errno_from_nix(e))
        }
    }
}

pub(crate) fn path_remove_directory(resolved: PathGet) -> Result<()> {
    use nix::errno;
    use nix::libc::{unlinkat, AT_REMOVEDIR};

    let path_cstr = CString::new(resolved.path().as_bytes()).map_err(|_| Error::EILSEQ)?;

    // nix doesn't expose unlinkat() yet
    match unsafe {
        unlinkat(
            resolved.dirfd().as_raw_fd(),
            path_cstr.as_ptr(),
            AT_REMOVEDIR,
        )
    } {
        0 => Ok(()),
        _ => Err(host_impl::errno_from_nix(errno::Errno::last())),
    }
}
