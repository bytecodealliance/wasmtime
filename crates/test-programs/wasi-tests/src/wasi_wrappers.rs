//! Minimal wrappers around WASI functions to allow use of `&str` rather than
//! pointer-length pairs.
//!
//! Where possible, we use the idiomatic wasi_unstable wrappers rather than the
//! raw interfaces, however for functions with out parameters, we use the raw
//! interfaces so that we can test whether they are stored to. In the future,
//! WASI should switch to multi-value and eliminate out parameters altogether.

use wasi::wasi_unstable;

pub unsafe fn wasi_path_create_directory(
    dir_fd: wasi_unstable::Fd,
    dir_name: &str,
) -> Result<(), wasi_unstable::Error> {
    wasi_unstable::path_create_directory(dir_fd, dir_name.as_bytes())
}

pub unsafe fn wasi_path_remove_directory(
    dir_fd: wasi_unstable::Fd,
    dir_name: &str,
) -> Result<(), wasi_unstable::Error> {
    wasi_unstable::path_remove_directory(dir_fd, dir_name.as_bytes())
}

pub unsafe fn wasi_path_unlink_file(
    dir_fd: wasi_unstable::Fd,
    file_name: &str,
) -> Result<(), wasi_unstable::Error> {
    wasi_unstable::path_unlink_file(dir_fd, file_name.as_bytes())
}

#[allow(clippy::too_many_arguments)]
pub unsafe fn wasi_path_open(
    dirfd: wasi_unstable::Fd,
    dirflags: wasi_unstable::LookupFlags,
    path: &str,
    oflags: wasi_unstable::OFlags,
    fs_rights_base: wasi_unstable::Rights,
    fs_rights_inheriting: wasi_unstable::Rights,
    fs_flags: wasi_unstable::FdFlags,
    fd: &mut wasi_unstable::Fd,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_path_open(
        dirfd,
        dirflags,
        path.as_ptr(),
        path.len(),
        oflags,
        fs_rights_base,
        fs_rights_inheriting,
        fs_flags,
        fd,
    )
}

pub unsafe fn wasi_path_symlink(
    old_path: &str,
    dirfd: wasi_unstable::Fd,
    new_path: &str,
) -> Result<(), wasi_unstable::Error> {
    wasi_unstable::path_symlink(old_path.as_bytes(), dirfd, new_path.as_bytes())
}

pub unsafe fn wasi_path_link(
    old_fd: wasi_unstable::Fd,
    old_flags: wasi_unstable::LookupFlags,
    old_path: &str,
    new_fd: wasi_unstable::Fd,
    new_path: &str,
) -> Result<(), wasi_unstable::Error> {
    wasi_unstable::path_link(
        old_fd,
        old_flags,
        old_path.as_bytes(),
        new_fd,
        new_path.as_bytes(),
    )
}

pub unsafe fn wasi_path_readlink(
    dirfd: wasi_unstable::Fd,
    path: &str,
    buf: &mut [u8],
    bufused: &mut usize,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_path_readlink(
        dirfd,
        path.as_ptr(),
        path.len(),
        buf.as_mut_ptr(),
        buf.len(),
        bufused,
    )
}

pub unsafe fn wasi_path_rename(
    old_dirfd: wasi_unstable::Fd,
    old_path: &str,
    new_dirfd: wasi_unstable::Fd,
    new_path: &str,
) -> Result<(), wasi_unstable::Error> {
    wasi_unstable::path_rename(
        old_dirfd,
        old_path.as_bytes(),
        new_dirfd,
        new_path.as_bytes(),
    )
}

pub unsafe fn wasi_fd_fdstat_get(
    fd: wasi_unstable::Fd,
    fdstat: &mut wasi_unstable::FdStat,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_fd_fdstat_get(fd, fdstat)
}

pub unsafe fn wasi_fd_seek(
    fd: wasi_unstable::Fd,
    offset: wasi_unstable::FileDelta,
    whence: wasi_unstable::Whence,
    newoffset: &mut wasi_unstable::FileSize,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_fd_seek(fd, offset, whence, newoffset)
}

pub unsafe fn wasi_fd_tell(
    fd: wasi_unstable::Fd,
    offset: &mut wasi_unstable::FileSize,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_fd_tell(fd, offset)
}

pub unsafe fn wasi_clock_time_get(
    clock_id: wasi_unstable::ClockId,
    precision: wasi_unstable::Timestamp,
    time: &mut wasi_unstable::Timestamp,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_clock_time_get(clock_id, precision, time)
}

pub unsafe fn wasi_fd_filestat_get(
    fd: wasi_unstable::Fd,
    filestat: &mut wasi_unstable::FileStat,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_fd_filestat_get(fd, filestat)
}

pub unsafe fn wasi_fd_write(
    fd: wasi_unstable::Fd,
    iovs: &[wasi_unstable::CIoVec],
    nwritten: &mut libc::size_t,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_fd_write(fd, iovs.as_ptr(), iovs.len(), nwritten)
}

pub unsafe fn wasi_fd_read(
    fd: wasi_unstable::Fd,
    iovs: &[wasi_unstable::IoVec],
    nread: &mut libc::size_t,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_fd_read(fd, iovs.as_ptr(), iovs.len(), nread)
}

pub unsafe fn wasi_fd_pread(
    fd: wasi_unstable::Fd,
    iovs: &[wasi_unstable::IoVec],
    offset: wasi_unstable::FileSize,
    nread: &mut usize,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_fd_pread(fd, iovs.as_ptr(), iovs.len(), offset, nread)
}

pub unsafe fn wasi_fd_pwrite(
    fd: wasi_unstable::Fd,
    iovs: &mut [wasi_unstable::CIoVec],
    offset: wasi_unstable::FileSize,
    nwritten: &mut usize,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_fd_pwrite(fd, iovs.as_ptr(), iovs.len(), offset, nwritten)
}

pub unsafe fn wasi_path_filestat_get(
    fd: wasi_unstable::Fd,
    dirflags: wasi_unstable::LookupFlags,
    path: &str,
    path_len: usize,
    filestat: &mut wasi_unstable::FileStat,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_path_filestat_get(fd, dirflags, path.as_ptr(), path_len, filestat)
}

pub unsafe fn wasi_path_filestat_set_times(
    fd: wasi_unstable::Fd,
    dirflags: wasi_unstable::LookupFlags,
    path: &str,
    st_atim: wasi_unstable::Timestamp,
    st_mtim: wasi_unstable::Timestamp,
    fst_flags: wasi_unstable::FstFlags,
) -> Result<(), wasi_unstable::Error> {
    wasi_unstable::path_filestat_set_times(
        fd,
        dirflags,
        path.as_bytes(),
        st_atim,
        st_mtim,
        fst_flags,
    )
}

pub unsafe fn wasi_fd_readdir(
    fd: wasi_unstable::Fd,
    buf: &mut [u8],
    buf_len: usize,
    cookie: wasi_unstable::DirCookie,
    buf_used: &mut usize,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_fd_readdir(
        fd,
        buf.as_mut_ptr() as *mut libc::c_void,
        buf_len,
        cookie,
        buf_used,
    )
}

pub unsafe fn wasi_fd_advise(
    fd: wasi_unstable::Fd,
    offset: wasi_unstable::FileSize,
    len: wasi_unstable::FileSize,
    advice: wasi_unstable::Advice,
) -> wasi_unstable::Errno {
    wasi_unstable::raw::__wasi_fd_advise(fd, offset, len, advice)
}
