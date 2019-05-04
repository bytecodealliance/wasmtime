#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/wasi_host.rs"));

pub type void = ::std::os::raw::c_void;

pub unsafe fn ciovec_to_nix<'a>(ciovec: &'a __wasi_ciovec_t) -> nix::sys::uio::IoVec<&'a [u8]> {
    let slice = std::slice::from_raw_parts(ciovec.buf as *const u8, ciovec.buf_len);
    nix::sys::uio::IoVec::from_slice(slice)
}

pub unsafe fn ciovec_to_nix_mut<'a>(
    ciovec: &'a mut __wasi_ciovec_t,
) -> nix::sys::uio::IoVec<&'a mut [u8]> {
    let slice = std::slice::from_raw_parts_mut(ciovec.buf as *mut u8, ciovec.buf_len);
    nix::sys::uio::IoVec::from_mut_slice(slice)
}

pub fn errno_from_nix(errno: nix::errno::Errno) -> __wasi_errno_t {
    let e = match errno {
        nix::errno::Errno::EPERM => __WASI_EPERM,
        nix::errno::Errno::ENOENT => __WASI_ENOENT,
        nix::errno::Errno::ESRCH => __WASI_ESRCH,
        nix::errno::Errno::EINTR => __WASI_EINTR,
        nix::errno::Errno::EIO => __WASI_EIO,
        nix::errno::Errno::ENXIO => __WASI_ENXIO,
        nix::errno::Errno::E2BIG => __WASI_E2BIG,
        nix::errno::Errno::ENOEXEC => __WASI_ENOEXEC,
        nix::errno::Errno::EBADF => __WASI_EBADF,
        nix::errno::Errno::ECHILD => __WASI_ECHILD,
        nix::errno::Errno::EAGAIN => __WASI_EAGAIN,
        nix::errno::Errno::ENOMEM => __WASI_ENOMEM,
        nix::errno::Errno::EACCES => __WASI_EACCES,
        nix::errno::Errno::EFAULT => __WASI_EFAULT,
        nix::errno::Errno::EBUSY => __WASI_EBUSY,
        nix::errno::Errno::EEXIST => __WASI_EEXIST,
        nix::errno::Errno::EXDEV => __WASI_EXDEV,
        nix::errno::Errno::ENODEV => __WASI_ENODEV,
        nix::errno::Errno::ENOTDIR => __WASI_ENOTDIR,
        nix::errno::Errno::EISDIR => __WASI_EISDIR,
        nix::errno::Errno::EINVAL => __WASI_EINVAL,
        nix::errno::Errno::ENFILE => __WASI_ENFILE,
        nix::errno::Errno::EMFILE => __WASI_EMFILE,
        nix::errno::Errno::ENOTTY => __WASI_ENOTTY,
        nix::errno::Errno::ETXTBSY => __WASI_ETXTBSY,
        nix::errno::Errno::EFBIG => __WASI_EFBIG,
        nix::errno::Errno::ENOSPC => __WASI_ENOSPC,
        nix::errno::Errno::ESPIPE => __WASI_ESPIPE,
        nix::errno::Errno::EROFS => __WASI_EROFS,
        nix::errno::Errno::EMLINK => __WASI_EMLINK,
        nix::errno::Errno::EPIPE => __WASI_EPIPE,
        nix::errno::Errno::EDOM => __WASI_EDOM,
        nix::errno::Errno::ERANGE => __WASI_ERANGE,
        nix::errno::Errno::EDEADLK => __WASI_EDEADLK,
        nix::errno::Errno::ENAMETOOLONG => __WASI_ENAMETOOLONG,
        nix::errno::Errno::ENOLCK => __WASI_ENOLCK,
        nix::errno::Errno::ENOSYS => __WASI_ENOSYS,
        nix::errno::Errno::ENOTEMPTY => __WASI_ENOTEMPTY,
        nix::errno::Errno::ELOOP => __WASI_ELOOP,
        nix::errno::Errno::ENOMSG => __WASI_ENOMSG,
        nix::errno::Errno::EIDRM => __WASI_EIDRM,
        nix::errno::Errno::ENOLINK => __WASI_ENOLINK,
        nix::errno::Errno::EPROTO => __WASI_EPROTO,
        nix::errno::Errno::EMULTIHOP => __WASI_EMULTIHOP,
        nix::errno::Errno::EBADMSG => __WASI_EBADMSG,
        nix::errno::Errno::EOVERFLOW => __WASI_EOVERFLOW,
        nix::errno::Errno::EILSEQ => __WASI_EILSEQ,
        nix::errno::Errno::ENOTSOCK => __WASI_ENOTSOCK,
        nix::errno::Errno::EDESTADDRREQ => __WASI_EDESTADDRREQ,
        nix::errno::Errno::EMSGSIZE => __WASI_EMSGSIZE,
        nix::errno::Errno::EPROTOTYPE => __WASI_EPROTOTYPE,
        nix::errno::Errno::ENOPROTOOPT => __WASI_ENOPROTOOPT,
        nix::errno::Errno::EPROTONOSUPPORT => __WASI_EPROTONOSUPPORT,
        nix::errno::Errno::EAFNOSUPPORT => __WASI_EAFNOSUPPORT,
        nix::errno::Errno::EADDRINUSE => __WASI_EADDRINUSE,
        nix::errno::Errno::EADDRNOTAVAIL => __WASI_EADDRNOTAVAIL,
        nix::errno::Errno::ENETDOWN => __WASI_ENETDOWN,
        nix::errno::Errno::ENETUNREACH => __WASI_ENETUNREACH,
        nix::errno::Errno::ENETRESET => __WASI_ENETRESET,
        nix::errno::Errno::ECONNABORTED => __WASI_ECONNABORTED,
        nix::errno::Errno::ECONNRESET => __WASI_ECONNRESET,
        nix::errno::Errno::ENOBUFS => __WASI_ENOBUFS,
        nix::errno::Errno::EISCONN => __WASI_EISCONN,
        nix::errno::Errno::ENOTCONN => __WASI_ENOTCONN,
        nix::errno::Errno::ETIMEDOUT => __WASI_ETIMEDOUT,
        nix::errno::Errno::ECONNREFUSED => __WASI_ECONNREFUSED,
        nix::errno::Errno::EHOSTUNREACH => __WASI_EHOSTUNREACH,
        nix::errno::Errno::EALREADY => __WASI_EALREADY,
        nix::errno::Errno::EINPROGRESS => __WASI_EINPROGRESS,
        nix::errno::Errno::ESTALE => __WASI_ESTALE,
        nix::errno::Errno::EDQUOT => __WASI_EDQUOT,
        nix::errno::Errno::ECANCELED => __WASI_ECANCELED,
        nix::errno::Errno::EOWNERDEAD => __WASI_EOWNERDEAD,
        nix::errno::Errno::ENOTRECOVERABLE => __WASI_ENOTRECOVERABLE,
        _ => __WASI_ENOSYS,
    };
    e as __wasi_errno_t
}

#[cfg(target_os = "linux")]
const O_RSYNC: nix::fcntl::OFlag = nix::fcntl::OFlag::O_RSYNC;

#[cfg(not(target_os = "linux"))]
const O_RSYNC: nix::fcntl::OFlag = nix::fcntl::OFlag::O_SYNC;

pub fn nix_from_fdflags(fdflags: __wasi_fdflags_t) -> nix::fcntl::OFlag {
    use nix::fcntl::OFlag;
    let mut nix_flags = OFlag::empty();
    if fdflags & (__WASI_FDFLAG_APPEND as __wasi_fdflags_t) != 0 {
        nix_flags.insert(OFlag::O_APPEND);
    }
    if fdflags & (__WASI_FDFLAG_DSYNC as __wasi_fdflags_t) != 0 {
        nix_flags.insert(OFlag::O_DSYNC);
    }
    if fdflags & (__WASI_FDFLAG_NONBLOCK as __wasi_fdflags_t) != 0 {
        nix_flags.insert(OFlag::O_NONBLOCK);
    }
    if fdflags & (__WASI_FDFLAG_RSYNC as __wasi_fdflags_t) != 0 {
        nix_flags.insert(O_RSYNC);
    }
    if fdflags & (__WASI_FDFLAG_SYNC as __wasi_fdflags_t) != 0 {
        nix_flags.insert(OFlag::O_SYNC);
    }
    nix_flags
}

pub fn fdflags_from_nix(oflags: nix::fcntl::OFlag) -> __wasi_fdflags_t {
    use nix::fcntl::OFlag;
    let mut fdflags = 0;
    if oflags.contains(OFlag::O_APPEND) {
        fdflags |= __WASI_FDFLAG_APPEND;
    }
    if oflags.contains(OFlag::O_DSYNC) {
        fdflags |= __WASI_FDFLAG_DSYNC;
    }
    if oflags.contains(OFlag::O_NONBLOCK) {
        fdflags |= __WASI_FDFLAG_NONBLOCK;
    }
    if oflags.contains(O_RSYNC) {
        fdflags |= __WASI_FDFLAG_RSYNC;
    }
    if oflags.contains(OFlag::O_SYNC) {
        fdflags |= __WASI_FDFLAG_SYNC;
    }
    fdflags as __wasi_fdflags_t
}

pub fn nix_from_oflags(oflags: __wasi_oflags_t) -> nix::fcntl::OFlag {
    use nix::fcntl::OFlag;
    let mut nix_flags = OFlag::empty();
    if oflags & (__WASI_O_CREAT as __wasi_oflags_t) != 0 {
        nix_flags.insert(OFlag::O_CREAT);
    }
    if oflags & (__WASI_O_DIRECTORY as __wasi_oflags_t) != 0 {
        nix_flags.insert(OFlag::O_DIRECTORY);
    }
    if oflags & (__WASI_O_EXCL as __wasi_oflags_t) != 0 {
        nix_flags.insert(OFlag::O_EXCL);
    }
    if oflags & (__WASI_O_TRUNC as __wasi_oflags_t) != 0 {
        nix_flags.insert(OFlag::O_TRUNC);
    }
    nix_flags
}

pub fn filetype_from_nix(sflags: nix::sys::stat::SFlag) -> __wasi_filetype_t {
    use nix::sys::stat::SFlag;
    if sflags.contains(SFlag::S_IFCHR) {
        __WASI_FILETYPE_CHARACTER_DEVICE as __wasi_filetype_t
    } else if sflags.contains(SFlag::S_IFBLK) {
        __WASI_FILETYPE_BLOCK_DEVICE as __wasi_filetype_t
    } else if sflags.contains(SFlag::S_IFIFO) | sflags.contains(SFlag::S_IFSOCK) {
        __WASI_FILETYPE_SOCKET_STREAM as __wasi_filetype_t
    } else if sflags.contains(SFlag::S_IFDIR) {
        __WASI_FILETYPE_DIRECTORY as __wasi_filetype_t
    } else if sflags.contains(SFlag::S_IFREG) {
        __WASI_FILETYPE_REGULAR_FILE as __wasi_filetype_t
    } else if sflags.contains(SFlag::S_IFLNK) {
        __WASI_FILETYPE_SYMBOLIC_LINK as __wasi_filetype_t
    } else {
        __WASI_FILETYPE_UNKNOWN as __wasi_filetype_t
    }
}

pub fn nix_from_filetype(sflags: __wasi_filetype_t) -> nix::sys::stat::SFlag {
    use nix::sys::stat::SFlag;
    let mut nix_sflags = SFlag::empty();
    if sflags & (__WASI_FILETYPE_CHARACTER_DEVICE as __wasi_filetype_t) != 0 {
        nix_sflags.insert(SFlag::S_IFCHR);
    }
    if sflags & (__WASI_FILETYPE_BLOCK_DEVICE as __wasi_filetype_t) != 0 {
        nix_sflags.insert(SFlag::S_IFBLK);
    }
    if sflags & (__WASI_FILETYPE_SOCKET_STREAM as __wasi_filetype_t) != 0 {
        nix_sflags.insert(SFlag::S_IFIFO);
        nix_sflags.insert(SFlag::S_IFSOCK);
    }
    if sflags & (__WASI_FILETYPE_DIRECTORY as __wasi_filetype_t) != 0 {
        nix_sflags.insert(SFlag::S_IFDIR);
    }
    if sflags & (__WASI_FILETYPE_REGULAR_FILE as __wasi_filetype_t) != 0 {
        nix_sflags.insert(SFlag::S_IFREG);
    }
    if sflags & (__WASI_FILETYPE_SYMBOLIC_LINK as __wasi_filetype_t) != 0 {
        nix_sflags.insert(SFlag::S_IFLNK);
    }
    nix_sflags
}

pub fn filestat_from_nix(filestat: nix::sys::stat::FileStat) -> __wasi_filestat_t {
    let filetype = nix::sys::stat::SFlag::from_bits_truncate(filestat.st_mode);
    __wasi_filestat_t {
        st_dev: filestat.st_dev as __wasi_device_t,
        st_ino: filestat.st_ino as __wasi_inode_t,
        st_nlink: filestat.st_nlink as __wasi_linkcount_t,
        st_size: filestat.st_size as __wasi_filesize_t,
        st_atim: filestat.st_atime as __wasi_timestamp_t,
        st_ctim: filestat.st_ctime as __wasi_timestamp_t,
        st_mtim: filestat.st_mtime as __wasi_timestamp_t,
        st_filetype: filetype_from_nix(filetype),
    }
}

// Rights sets from wasmtime-wasi sandboxed system primitives. Transcribed because bindgen can't
// parse the #defines.

pub const RIGHTS_ALL: __wasi_rights_t = (__WASI_RIGHT_FD_DATASYNC
    | __WASI_RIGHT_FD_READ
    | __WASI_RIGHT_FD_SEEK
    | __WASI_RIGHT_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHT_FD_SYNC
    | __WASI_RIGHT_FD_TELL
    | __WASI_RIGHT_FD_WRITE
    | __WASI_RIGHT_FD_ADVISE
    | __WASI_RIGHT_FD_ALLOCATE
    | __WASI_RIGHT_PATH_CREATE_DIRECTORY
    | __WASI_RIGHT_PATH_CREATE_FILE
    | __WASI_RIGHT_PATH_LINK_SOURCE
    | __WASI_RIGHT_PATH_LINK_TARGET
    | __WASI_RIGHT_PATH_OPEN
    | __WASI_RIGHT_FD_READDIR
    | __WASI_RIGHT_PATH_READLINK
    | __WASI_RIGHT_PATH_RENAME_SOURCE
    | __WASI_RIGHT_PATH_RENAME_TARGET
    | __WASI_RIGHT_PATH_FILESTAT_GET
    | __WASI_RIGHT_PATH_FILESTAT_SET_SIZE
    | __WASI_RIGHT_PATH_FILESTAT_SET_TIMES
    | __WASI_RIGHT_FD_FILESTAT_GET
    | __WASI_RIGHT_FD_FILESTAT_SET_SIZE
    | __WASI_RIGHT_FD_FILESTAT_SET_TIMES
    | __WASI_RIGHT_PATH_SYMLINK
    | __WASI_RIGHT_PATH_UNLINK_FILE
    | __WASI_RIGHT_PATH_REMOVE_DIRECTORY
    | __WASI_RIGHT_POLL_FD_READWRITE
    | __WASI_RIGHT_SOCK_SHUTDOWN) as __wasi_rights_t;

// Block and character device interaction is outside the scope of
// CloudABI. Simply allow everything.
pub const RIGHTS_BLOCK_DEVICE_BASE: __wasi_rights_t = RIGHTS_ALL;
pub const RIGHTS_BLOCK_DEVICE_INHERITING: __wasi_rights_t = RIGHTS_ALL;
pub const RIGHTS_CHARACTER_DEVICE_BASE: __wasi_rights_t = RIGHTS_ALL;
pub const RIGHTS_CHARACTER_DEVICE_INHERITING: __wasi_rights_t = RIGHTS_ALL;

// Only allow directory operations on directories. Directories can only
// yield file descriptors to other directories and files.
pub const RIGHTS_DIRECTORY_BASE: __wasi_rights_t = (__WASI_RIGHT_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHT_FD_SYNC
    | __WASI_RIGHT_FD_ADVISE
    | __WASI_RIGHT_PATH_CREATE_DIRECTORY
    | __WASI_RIGHT_PATH_CREATE_FILE
    | __WASI_RIGHT_PATH_LINK_SOURCE
    | __WASI_RIGHT_PATH_LINK_TARGET
    | __WASI_RIGHT_PATH_OPEN
    | __WASI_RIGHT_FD_READDIR
    | __WASI_RIGHT_PATH_READLINK
    | __WASI_RIGHT_PATH_RENAME_SOURCE
    | __WASI_RIGHT_PATH_RENAME_TARGET
    | __WASI_RIGHT_PATH_FILESTAT_GET
    | __WASI_RIGHT_PATH_FILESTAT_SET_SIZE
    | __WASI_RIGHT_PATH_FILESTAT_SET_TIMES
    | __WASI_RIGHT_FD_FILESTAT_GET
    | __WASI_RIGHT_FD_FILESTAT_SET_SIZE
    | __WASI_RIGHT_FD_FILESTAT_SET_TIMES
    | __WASI_RIGHT_PATH_SYMLINK
    | __WASI_RIGHT_PATH_UNLINK_FILE
    | __WASI_RIGHT_PATH_REMOVE_DIRECTORY
    | __WASI_RIGHT_POLL_FD_READWRITE)
    as __wasi_rights_t;
pub const RIGHTS_DIRECTORY_INHERITING: __wasi_rights_t =
    (RIGHTS_DIRECTORY_BASE | RIGHTS_REGULAR_FILE_BASE);

// Operations that apply to regular files.
pub const RIGHTS_REGULAR_FILE_BASE: __wasi_rights_t = (__WASI_RIGHT_FD_DATASYNC
    | __WASI_RIGHT_FD_READ
    | __WASI_RIGHT_FD_SEEK
    | __WASI_RIGHT_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHT_FD_SYNC
    | __WASI_RIGHT_FD_TELL
    | __WASI_RIGHT_FD_WRITE
    | __WASI_RIGHT_FD_ADVISE
    | __WASI_RIGHT_FD_ALLOCATE
    | __WASI_RIGHT_FD_FILESTAT_GET
    | __WASI_RIGHT_FD_FILESTAT_SET_SIZE
    | __WASI_RIGHT_FD_FILESTAT_SET_TIMES
    | __WASI_RIGHT_POLL_FD_READWRITE)
    as __wasi_rights_t;
pub const RIGHTS_REGULAR_FILE_INHERITING: __wasi_rights_t = 0;

// Operations that apply to shared memory objects.
pub const RIGHTS_SHARED_MEMORY_BASE: __wasi_rights_t = (__WASI_RIGHT_FD_READ
    | __WASI_RIGHT_FD_WRITE
    | __WASI_RIGHT_FD_FILESTAT_GET
    | __WASI_RIGHT_FD_FILESTAT_SET_SIZE)
    as __wasi_rights_t;
pub const RIGHTS_SHARED_MEMORY_INHERITING: __wasi_rights_t = 0;

// Operations that apply to sockets and socket pairs.
pub const RIGHTS_SOCKET_BASE: __wasi_rights_t = (__WASI_RIGHT_FD_READ
    | __WASI_RIGHT_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHT_FD_WRITE
    | __WASI_RIGHT_FD_FILESTAT_GET
    | __WASI_RIGHT_POLL_FD_READWRITE
    | __WASI_RIGHT_SOCK_SHUTDOWN)
    as __wasi_rights_t;
pub const RIGHTS_SOCKET_INHERITING: __wasi_rights_t = RIGHTS_ALL;

// Operations that apply to TTYs.
pub const RIGHTS_TTY_BASE: __wasi_rights_t = (__WASI_RIGHT_FD_READ
    | __WASI_RIGHT_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHT_FD_WRITE
    | __WASI_RIGHT_FD_FILESTAT_GET
    | __WASI_RIGHT_POLL_FD_READWRITE)
    as __wasi_rights_t;
pub const RIGHTS_TTY_INHERITING: __wasi_rights_t = 0;
