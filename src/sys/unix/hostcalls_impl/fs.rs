#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use super::fs_helpers::*;
use crate::helpers::systemtime_to_timestamp;
use crate::hostcalls_impl::PathGet;
use crate::sys::errno_from_ioerror;
use crate::sys::host_impl;
use crate::{host, wasm32, Result};
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
    file.read_at(buf, offset).map_err(errno_from_ioerror)
}

pub(crate) fn fd_pwrite(file: &File, buf: &[u8], offset: host::__wasi_filesize_t) -> Result<usize> {
    file.write_at(buf, offset).map_err(errno_from_ioerror)
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

pub(crate) fn fd_advise(
    file: &File,
    advice: host::__wasi_advice_t,
    offset: host::__wasi_filesize_t,
    len: host::__wasi_filesize_t,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use nix::libc::off_t;

        let host_advice = match advice {
            host::__WASI_ADVICE_DONTNEED => libc::POSIX_FADV_DONTNEED,
            host::__WASI_ADVICE_SEQUENTIAL => libc::POSIX_FADV_SEQUENTIAL,
            host::__WASI_ADVICE_WILLNEED => libc::POSIX_FADV_DONTNEED,
            host::__WASI_ADVICE_NOREUSE => libc::POSIX_FADV_NOREUSE,
            host::__WASI_ADVICE_RANDOM => libc::POSIX_FADV_RANDOM,
            host::__WASI_ADVICE_NORMAL => libc::POSIX_FADV_NORMAL,
            _ => return Err(host::__WASI_EINVAL),
        };
        let res = unsafe {
            libc::posix_fadvise(file.as_raw_fd(), offset as off_t, len as off_t, host_advice)
        };
        if res != 0 {
            return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (file, offset, len);
        match advice {
            host::__WASI_ADVICE_DONTNEED
            | host::__WASI_ADVICE_SEQUENTIAL
            | host::__WASI_ADVICE_WILLNEED
            | host::__WASI_ADVICE_NOREUSE
            | host::__WASI_ADVICE_RANDOM
            | host::__WASI_ADVICE_NORMAL => {}
            _ => return Err(host::__WASI_EINVAL),
        }
    }

    Ok(())
}

pub(crate) fn path_create_directory(resolved: PathGet) -> Result<()> {
    use nix::libc::mkdirat;
    let path_cstr = CString::new(resolved.path().as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;
    // nix doesn't expose mkdirat() yet
    match unsafe { mkdirat(resolved.dirfd().as_raw_fd(), path_cstr.as_ptr(), 0o777) } {
        0 => Ok(()),
        _ => Err(host_impl::errno_from_nix(nix::errno::Errno::last())),
    }
}

pub(crate) fn path_link(resolved_old: PathGet, resolved_new: PathGet) -> Result<()> {
    use nix::libc::linkat;
    let old_path_cstr =
        CString::new(resolved_old.path().as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;
    let new_path_cstr =
        CString::new(resolved_new.path().as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

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
                            return Err(host::__WASI_ENOTSUP);
                        } else {
                            return Err(host::__WASI_ENXIO);
                        }
                    } else {
                        return Err(host::__WASI_ENXIO);
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
                            return Err(host::__WASI_ELOOP);
                        }
                    }
                    return Err(host::__WASI_ENOTDIR);
                }
                // FreeBSD returns EMLINK instead of ELOOP when using O_NOFOLLOW on
                // a symlink.
                Some(Errno::EMLINK) if !(nix_all_oflags & OFlag::O_NOFOLLOW).is_empty() => {
                    return Err(host::__WASI_ELOOP);
                }
                Some(e) => return Err(host_impl::errno_from_nix(e)),
                None => return Err(host::__WASI_ENOSYS),
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
    use libc::{dirent, fdopendir, memcpy, readdir_r, seekdir};

    let host_buf_ptr = host_buf.as_mut_ptr();
    let host_buf_len = host_buf.len();
    let dir = unsafe { fdopendir(fd.as_raw_fd()) };
    if dir.is_null() {
        return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
    }
    if cookie != wasm32::__WASI_DIRCOOKIE_START {
        unsafe { seekdir(dir, cookie as c_long) };
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
        let entry: wasm32::__wasi_dirent_t = host_impl::dirent_from_host(&unsafe { *host_entry })?;
        let name_len = entry.d_namlen as usize;
        let required_space = std::mem::size_of_val(&entry) + name_len;
        if required_space > left {
            break;
        }
        unsafe {
            let ptr = host_buf_ptr.offset(host_buf_offset as isize) as *mut c_void
                as *mut wasm32::__wasi_dirent_t;
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
    let path_cstr = CString::new(resolved.path().as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

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
    let old_path_cstr =
        CString::new(resolved_old.path().as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;
    let new_path_cstr =
        CString::new(resolved_new.path().as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

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

    let metadata = file.metadata().map_err(errno_from_ioerror)?;
    Ok(host::__wasi_filestat_t {
        st_dev: metadata.dev(),
        st_ino: metadata.ino(),
        st_nlink: metadata
            .nlink()
            .try_into()
            .map_err(|_| host::__WASI_EOVERFLOW)?, // u64 doesn't fit into u32
        st_size: metadata.len(),
        st_atim: metadata
            .accessed()
            .map_err(errno_from_ioerror)
            .and_then(systemtime_to_timestamp)?,
        st_ctim: metadata
            .ctime()
            .try_into()
            .map_err(|_| host::__WASI_EOVERFLOW)?, // i64 doesn't fit into u64
        st_mtim: metadata
            .modified()
            .map_err(errno_from_ioerror)
            .and_then(systemtime_to_timestamp)?,
        st_filetype: filetype(&metadata),
    })
}

fn filetype(metadata: &Metadata) -> host::__wasi_filetype_t {
    use std::os::unix::fs::FileTypeExt;
    let ftype = metadata.file_type();
    if ftype.is_file() {
        host::__WASI_FILETYPE_REGULAR_FILE
    } else if ftype.is_dir() {
        host::__WASI_FILETYPE_DIRECTORY
    } else if ftype.is_symlink() {
        host::__WASI_FILETYPE_SYMBOLIC_LINK
    } else if ftype.is_char_device() {
        host::__WASI_FILETYPE_CHARACTER_DEVICE
    } else if ftype.is_block_device() {
        host::__WASI_FILETYPE_BLOCK_DEVICE
    } else if ftype.is_socket() || ftype.is_fifo() {
        // TODO we should use getsockopt to find out if it's
        // SOCKET_STREAM or SOCKET_DGRAM
        host::__WASI_FILETYPE_SOCKET_STREAM
    } else {
        host::__WASI_FILETYPE_UNKNOWN
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
    mut st_mtim: host::__wasi_timestamp_t,
    fst_flags: host::__wasi_fstflags_t,
) -> Result<()> {
    use nix::sys::time::{TimeSpec, TimeValLike};

    let atflags = match dirflags {
        wasm32::__WASI_LOOKUP_SYMLINK_FOLLOW => 0,
        _ => libc::AT_SYMLINK_NOFOLLOW,
    };
    if fst_flags & host::__WASI_FILESTAT_SET_MTIM_NOW != 0 {
        let clock_id = libc::CLOCK_REALTIME;
        let mut timespec = unsafe { std::mem::uninitialized::<libc::timespec>() };
        let res = unsafe { libc::clock_gettime(clock_id, &mut timespec as *mut libc::timespec) };
        if res != 0 {
            return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
        }
        st_mtim = (timespec.tv_sec as host::__wasi_timestamp_t)
            .checked_mul(1_000_000_000)
            .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
            .ok_or(host::__WASI_EOVERFLOW)?;
    }
    let ts_atime = match fst_flags {
        f if f & host::__WASI_FILESTAT_SET_ATIM_NOW != 0 => libc::timespec {
            tv_sec: 0,
            tv_nsec: utime_now(),
        },
        f if f & host::__WASI_FILESTAT_SET_ATIM != 0 => {
            *TimeSpec::nanoseconds(st_atim as i64).as_ref()
        }
        _ => libc::timespec {
            tv_sec: 0,
            tv_nsec: utime_omit(),
        },
    };
    let ts_mtime = *TimeSpec::nanoseconds(st_mtim as i64).as_ref();
    let times = [ts_atime, ts_mtime];

    let path_cstr = CString::new(resolved.path().as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

    let res = unsafe {
        libc::utimensat(
            resolved.dirfd().as_raw_fd(),
            path_cstr.as_ptr(),
            times.as_ptr(),
            atflags,
        )
    };
    if res != 0 {
        Err(host_impl::errno_from_nix(nix::errno::Errno::last()))
    } else {
        Ok(())
    }
}

pub(crate) fn path_symlink(old_path: &str, resolved: PathGet) -> Result<()> {
    use nix::libc::symlinkat;

    let old_path_cstr = CString::new(old_path.as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;
    let new_path_cstr =
        CString::new(resolved.path().as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

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

    let path_cstr = CString::new(resolved.path().as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

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

    let path_cstr = CString::new(resolved.path().as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

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
