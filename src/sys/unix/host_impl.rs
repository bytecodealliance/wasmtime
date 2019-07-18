//! WASI host types specific to *nix host.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::{host, memory, wasm32, Result};
use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;

pub fn errno_from_nix(errno: nix::errno::Errno) -> host::__wasi_errno_t {
    match errno {
        nix::errno::Errno::EPERM => host::__WASI_EPERM,
        nix::errno::Errno::ENOENT => host::__WASI_ENOENT,
        nix::errno::Errno::ESRCH => host::__WASI_ESRCH,
        nix::errno::Errno::EINTR => host::__WASI_EINTR,
        nix::errno::Errno::EIO => host::__WASI_EIO,
        nix::errno::Errno::ENXIO => host::__WASI_ENXIO,
        nix::errno::Errno::E2BIG => host::__WASI_E2BIG,
        nix::errno::Errno::ENOEXEC => host::__WASI_ENOEXEC,
        nix::errno::Errno::EBADF => host::__WASI_EBADF,
        nix::errno::Errno::ECHILD => host::__WASI_ECHILD,
        nix::errno::Errno::EAGAIN => host::__WASI_EAGAIN,
        nix::errno::Errno::ENOMEM => host::__WASI_ENOMEM,
        nix::errno::Errno::EACCES => host::__WASI_EACCES,
        nix::errno::Errno::EFAULT => host::__WASI_EFAULT,
        nix::errno::Errno::EBUSY => host::__WASI_EBUSY,
        nix::errno::Errno::EEXIST => host::__WASI_EEXIST,
        nix::errno::Errno::EXDEV => host::__WASI_EXDEV,
        nix::errno::Errno::ENODEV => host::__WASI_ENODEV,
        nix::errno::Errno::ENOTDIR => host::__WASI_ENOTDIR,
        nix::errno::Errno::EISDIR => host::__WASI_EISDIR,
        nix::errno::Errno::EINVAL => host::__WASI_EINVAL,
        nix::errno::Errno::ENFILE => host::__WASI_ENFILE,
        nix::errno::Errno::EMFILE => host::__WASI_EMFILE,
        nix::errno::Errno::ENOTTY => host::__WASI_ENOTTY,
        nix::errno::Errno::ETXTBSY => host::__WASI_ETXTBSY,
        nix::errno::Errno::EFBIG => host::__WASI_EFBIG,
        nix::errno::Errno::ENOSPC => host::__WASI_ENOSPC,
        nix::errno::Errno::ESPIPE => host::__WASI_ESPIPE,
        nix::errno::Errno::EROFS => host::__WASI_EROFS,
        nix::errno::Errno::EMLINK => host::__WASI_EMLINK,
        nix::errno::Errno::EPIPE => host::__WASI_EPIPE,
        nix::errno::Errno::EDOM => host::__WASI_EDOM,
        nix::errno::Errno::ERANGE => host::__WASI_ERANGE,
        nix::errno::Errno::EDEADLK => host::__WASI_EDEADLK,
        nix::errno::Errno::ENAMETOOLONG => host::__WASI_ENAMETOOLONG,
        nix::errno::Errno::ENOLCK => host::__WASI_ENOLCK,
        nix::errno::Errno::ENOSYS => host::__WASI_ENOSYS,
        nix::errno::Errno::ENOTEMPTY => host::__WASI_ENOTEMPTY,
        nix::errno::Errno::ELOOP => host::__WASI_ELOOP,
        nix::errno::Errno::ENOMSG => host::__WASI_ENOMSG,
        nix::errno::Errno::EIDRM => host::__WASI_EIDRM,
        nix::errno::Errno::ENOLINK => host::__WASI_ENOLINK,
        nix::errno::Errno::EPROTO => host::__WASI_EPROTO,
        nix::errno::Errno::EMULTIHOP => host::__WASI_EMULTIHOP,
        nix::errno::Errno::EBADMSG => host::__WASI_EBADMSG,
        nix::errno::Errno::EOVERFLOW => host::__WASI_EOVERFLOW,
        nix::errno::Errno::EILSEQ => host::__WASI_EILSEQ,
        nix::errno::Errno::ENOTSOCK => host::__WASI_ENOTSOCK,
        nix::errno::Errno::EDESTADDRREQ => host::__WASI_EDESTADDRREQ,
        nix::errno::Errno::EMSGSIZE => host::__WASI_EMSGSIZE,
        nix::errno::Errno::EPROTOTYPE => host::__WASI_EPROTOTYPE,
        nix::errno::Errno::ENOPROTOOPT => host::__WASI_ENOPROTOOPT,
        nix::errno::Errno::EPROTONOSUPPORT => host::__WASI_EPROTONOSUPPORT,
        nix::errno::Errno::EAFNOSUPPORT => host::__WASI_EAFNOSUPPORT,
        nix::errno::Errno::EADDRINUSE => host::__WASI_EADDRINUSE,
        nix::errno::Errno::EADDRNOTAVAIL => host::__WASI_EADDRNOTAVAIL,
        nix::errno::Errno::ENETDOWN => host::__WASI_ENETDOWN,
        nix::errno::Errno::ENETUNREACH => host::__WASI_ENETUNREACH,
        nix::errno::Errno::ENETRESET => host::__WASI_ENETRESET,
        nix::errno::Errno::ECONNABORTED => host::__WASI_ECONNABORTED,
        nix::errno::Errno::ECONNRESET => host::__WASI_ECONNRESET,
        nix::errno::Errno::ENOBUFS => host::__WASI_ENOBUFS,
        nix::errno::Errno::EISCONN => host::__WASI_EISCONN,
        nix::errno::Errno::ENOTCONN => host::__WASI_ENOTCONN,
        nix::errno::Errno::ETIMEDOUT => host::__WASI_ETIMEDOUT,
        nix::errno::Errno::ECONNREFUSED => host::__WASI_ECONNREFUSED,
        nix::errno::Errno::EHOSTUNREACH => host::__WASI_EHOSTUNREACH,
        nix::errno::Errno::EALREADY => host::__WASI_EALREADY,
        nix::errno::Errno::EINPROGRESS => host::__WASI_EINPROGRESS,
        nix::errno::Errno::ESTALE => host::__WASI_ESTALE,
        nix::errno::Errno::EDQUOT => host::__WASI_EDQUOT,
        nix::errno::Errno::ECANCELED => host::__WASI_ECANCELED,
        nix::errno::Errno::EOWNERDEAD => host::__WASI_EOWNERDEAD,
        nix::errno::Errno::ENOTRECOVERABLE => host::__WASI_ENOTRECOVERABLE,
        _ => host::__WASI_ENOSYS,
    }
}

#[cfg(target_os = "linux")]
pub const O_RSYNC: nix::fcntl::OFlag = nix::fcntl::OFlag::O_RSYNC;

#[cfg(not(target_os = "linux"))]
pub const O_RSYNC: nix::fcntl::OFlag = nix::fcntl::OFlag::O_SYNC;

pub fn nix_from_fdflags(fdflags: host::__wasi_fdflags_t) -> nix::fcntl::OFlag {
    use nix::fcntl::OFlag;
    let mut nix_flags = OFlag::empty();
    if fdflags & host::__WASI_FDFLAG_APPEND != 0 {
        nix_flags.insert(OFlag::O_APPEND);
    }
    if fdflags & host::__WASI_FDFLAG_DSYNC != 0 {
        nix_flags.insert(OFlag::O_DSYNC);
    }
    if fdflags & host::__WASI_FDFLAG_NONBLOCK != 0 {
        nix_flags.insert(OFlag::O_NONBLOCK);
    }
    if fdflags & host::__WASI_FDFLAG_RSYNC != 0 {
        nix_flags.insert(O_RSYNC);
    }
    if fdflags & host::__WASI_FDFLAG_SYNC != 0 {
        nix_flags.insert(OFlag::O_SYNC);
    }
    nix_flags
}

pub fn fdflags_from_nix(oflags: nix::fcntl::OFlag) -> host::__wasi_fdflags_t {
    use nix::fcntl::OFlag;
    let mut fdflags = 0;
    if oflags.contains(OFlag::O_APPEND) {
        fdflags |= host::__WASI_FDFLAG_APPEND;
    }
    if oflags.contains(OFlag::O_DSYNC) {
        fdflags |= host::__WASI_FDFLAG_DSYNC;
    }
    if oflags.contains(OFlag::O_NONBLOCK) {
        fdflags |= host::__WASI_FDFLAG_NONBLOCK;
    }
    if oflags.contains(O_RSYNC) {
        fdflags |= host::__WASI_FDFLAG_RSYNC;
    }
    if oflags.contains(OFlag::O_SYNC) {
        fdflags |= host::__WASI_FDFLAG_SYNC;
    }
    fdflags
}

pub fn nix_from_oflags(oflags: host::__wasi_oflags_t) -> nix::fcntl::OFlag {
    use nix::fcntl::OFlag;
    let mut nix_flags = OFlag::empty();
    if oflags & host::__WASI_O_CREAT != 0 {
        nix_flags.insert(OFlag::O_CREAT);
    }
    if oflags & host::__WASI_O_DIRECTORY != 0 {
        nix_flags.insert(OFlag::O_DIRECTORY);
    }
    if oflags & host::__WASI_O_EXCL != 0 {
        nix_flags.insert(OFlag::O_EXCL);
    }
    if oflags & host::__WASI_O_TRUNC != 0 {
        nix_flags.insert(OFlag::O_TRUNC);
    }
    nix_flags
}

pub fn filetype_from_nix(sflags: nix::sys::stat::SFlag) -> host::__wasi_filetype_t {
    use nix::sys::stat::SFlag;
    if sflags.contains(SFlag::S_IFCHR) {
        host::__WASI_FILETYPE_CHARACTER_DEVICE
    } else if sflags.contains(SFlag::S_IFBLK) {
        host::__WASI_FILETYPE_BLOCK_DEVICE
    } else if sflags.contains(SFlag::S_IFIFO) | sflags.contains(SFlag::S_IFSOCK) {
        host::__WASI_FILETYPE_SOCKET_STREAM
    } else if sflags.contains(SFlag::S_IFDIR) {
        host::__WASI_FILETYPE_DIRECTORY
    } else if sflags.contains(SFlag::S_IFREG) {
        host::__WASI_FILETYPE_REGULAR_FILE
    } else if sflags.contains(SFlag::S_IFLNK) {
        host::__WASI_FILETYPE_SYMBOLIC_LINK
    } else {
        host::__WASI_FILETYPE_UNKNOWN
    }
}

pub fn nix_from_filetype(sflags: host::__wasi_filetype_t) -> nix::sys::stat::SFlag {
    use nix::sys::stat::SFlag;
    let mut nix_sflags = SFlag::empty();
    if sflags & host::__WASI_FILETYPE_CHARACTER_DEVICE != 0 {
        nix_sflags.insert(SFlag::S_IFCHR);
    }
    if sflags & host::__WASI_FILETYPE_BLOCK_DEVICE != 0 {
        nix_sflags.insert(SFlag::S_IFBLK);
    }
    if sflags & host::__WASI_FILETYPE_SOCKET_STREAM != 0 {
        nix_sflags.insert(SFlag::S_IFIFO);
        nix_sflags.insert(SFlag::S_IFSOCK);
    }
    if sflags & host::__WASI_FILETYPE_DIRECTORY != 0 {
        nix_sflags.insert(SFlag::S_IFDIR);
    }
    if sflags & host::__WASI_FILETYPE_REGULAR_FILE != 0 {
        nix_sflags.insert(SFlag::S_IFREG);
    }
    if sflags & host::__WASI_FILETYPE_SYMBOLIC_LINK != 0 {
        nix_sflags.insert(SFlag::S_IFLNK);
    }
    nix_sflags
}

pub fn filestat_from_nix(filestat: nix::sys::stat::FileStat) -> Result<host::__wasi_filestat_t> {
    use std::convert::TryFrom;

    let filetype = nix::sys::stat::SFlag::from_bits_truncate(filestat.st_mode);
    let dev =
        host::__wasi_device_t::try_from(filestat.st_dev).map_err(|_| host::__WASI_EOVERFLOW)?;
    let ino =
        host::__wasi_inode_t::try_from(filestat.st_ino).map_err(|_| host::__WASI_EOVERFLOW)?;

    Ok(host::__wasi_filestat_t {
        st_dev: dev,
        st_ino: ino,
        st_nlink: filestat.st_nlink as host::__wasi_linkcount_t,
        st_size: filestat.st_size as host::__wasi_filesize_t,
        st_atim: filestat.st_atime as host::__wasi_timestamp_t,
        st_ctim: filestat.st_ctime as host::__wasi_timestamp_t,
        st_mtim: filestat.st_mtime as host::__wasi_timestamp_t,
        st_filetype: filetype_from_nix(filetype),
    })
}

#[cfg(target_os = "linux")]
pub fn dirent_from_host(host_entry: &nix::libc::dirent) -> Result<wasm32::__wasi_dirent_t> {
    let mut entry = unsafe { std::mem::zeroed::<wasm32::__wasi_dirent_t>() };
    let d_namlen = unsafe { std::ffi::CStr::from_ptr(host_entry.d_name.as_ptr()) }
        .to_bytes()
        .len();
    if d_namlen > u32::max_value() as usize {
        return Err(host::__WASI_EIO);
    }
    entry.d_ino = memory::enc_inode(host_entry.d_ino);
    entry.d_next = memory::enc_dircookie(host_entry.d_off as u64);
    entry.d_namlen = memory::enc_u32(d_namlen as u32);
    entry.d_type = memory::enc_filetype(host_entry.d_type);
    Ok(entry)
}

#[cfg(not(target_os = "linux"))]
pub fn dirent_from_host(host_entry: &nix::libc::dirent) -> Result<wasm32::__wasi_dirent_t> {
    let mut entry = unsafe { std::mem::zeroed::<wasm32::__wasi_dirent_t>() };
    entry.d_ino = memory::enc_inode(host_entry.d_ino);
    entry.d_next = memory::enc_dircookie(host_entry.d_seekoff);
    entry.d_namlen = memory::enc_u32(u32::from(host_entry.d_namlen));
    entry.d_type = memory::enc_filetype(host_entry.d_type);
    Ok(entry)
}

/// Creates owned WASI path from OS string.
///
/// NB WASI spec requires OS string to be valid UTF-8. Otherwise,
/// `__WASI_EILSEQ` error is returned.
pub fn path_from_host<S: AsRef<OsStr>>(s: S) -> Result<String> {
    host::path_from_slice(s.as_ref().as_bytes()).map(String::from)
}
