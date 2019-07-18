//! WASI host types as defined in host. This file was originally generated
//! by running bindgen over wasi/core.h, and the content
//! still largely reflects that.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
use crate::Result;
use std::{io, slice, str};

pub type void = ::std::os::raw::c_void;

pub type __wasi_advice_t = u8;
pub const __WASI_ADVICE_NORMAL: __wasi_advice_t = 0;
pub const __WASI_ADVICE_SEQUENTIAL: __wasi_advice_t = 1;
pub const __WASI_ADVICE_RANDOM: __wasi_advice_t = 2;
pub const __WASI_ADVICE_WILLNEED: __wasi_advice_t = 3;
pub const __WASI_ADVICE_DONTNEED: __wasi_advice_t = 4;
pub const __WASI_ADVICE_NOREUSE: __wasi_advice_t = 5;

pub type __wasi_clockid_t = u32;
pub const __WASI_CLOCK_REALTIME: __wasi_clockid_t = 0;
pub const __WASI_CLOCK_MONOTONIC: __wasi_clockid_t = 1;
pub const __WASI_CLOCK_PROCESS_CPUTIME_ID: __wasi_clockid_t = 2;
pub const __WASI_CLOCK_THREAD_CPUTIME_ID: __wasi_clockid_t = 3;

pub type __wasi_device_t = u64;

pub type __wasi_dircookie_t = u64;
pub const __WASI_DIRCOOKIE_START: __wasi_dircookie_t = 0;

// WASI error codes
pub type __wasi_errno_t = u16;
pub const __WASI_ESUCCESS: __wasi_errno_t = 0;
pub const __WASI_E2BIG: __wasi_errno_t = 1;
pub const __WASI_EACCES: __wasi_errno_t = 2;
pub const __WASI_EADDRINUSE: __wasi_errno_t = 3;
pub const __WASI_EADDRNOTAVAIL: __wasi_errno_t = 4;
pub const __WASI_EAFNOSUPPORT: __wasi_errno_t = 5;
pub const __WASI_EAGAIN: __wasi_errno_t = 6;
pub const __WASI_EALREADY: __wasi_errno_t = 7;
pub const __WASI_EBADF: __wasi_errno_t = 8;
pub const __WASI_EBADMSG: __wasi_errno_t = 9;
pub const __WASI_EBUSY: __wasi_errno_t = 10;
pub const __WASI_ECANCELED: __wasi_errno_t = 11;
pub const __WASI_ECHILD: __wasi_errno_t = 12;
pub const __WASI_ECONNABORTED: __wasi_errno_t = 13;
pub const __WASI_ECONNREFUSED: __wasi_errno_t = 14;
pub const __WASI_ECONNRESET: __wasi_errno_t = 15;
pub const __WASI_EDEADLK: __wasi_errno_t = 16;
pub const __WASI_EDESTADDRREQ: __wasi_errno_t = 17;
pub const __WASI_EDOM: __wasi_errno_t = 18;
pub const __WASI_EDQUOT: __wasi_errno_t = 19;
pub const __WASI_EEXIST: __wasi_errno_t = 20;
pub const __WASI_EFAULT: __wasi_errno_t = 21;
pub const __WASI_EFBIG: __wasi_errno_t = 22;
pub const __WASI_EHOSTUNREACH: __wasi_errno_t = 23;
pub const __WASI_EIDRM: __wasi_errno_t = 24;
pub const __WASI_EILSEQ: __wasi_errno_t = 25;
pub const __WASI_EINPROGRESS: __wasi_errno_t = 26;
pub const __WASI_EINTR: __wasi_errno_t = 27;
pub const __WASI_EINVAL: __wasi_errno_t = 28;
pub const __WASI_EIO: __wasi_errno_t = 29;
pub const __WASI_EISCONN: __wasi_errno_t = 30;
pub const __WASI_EISDIR: __wasi_errno_t = 31;
pub const __WASI_ELOOP: __wasi_errno_t = 32;
pub const __WASI_EMFILE: __wasi_errno_t = 33;
pub const __WASI_EMLINK: __wasi_errno_t = 34;
pub const __WASI_EMSGSIZE: __wasi_errno_t = 35;
pub const __WASI_EMULTIHOP: __wasi_errno_t = 36;
pub const __WASI_ENAMETOOLONG: __wasi_errno_t = 37;
pub const __WASI_ENETDOWN: __wasi_errno_t = 38;
pub const __WASI_ENETRESET: __wasi_errno_t = 39;
pub const __WASI_ENETUNREACH: __wasi_errno_t = 40;
pub const __WASI_ENFILE: __wasi_errno_t = 41;
pub const __WASI_ENOBUFS: __wasi_errno_t = 42;
pub const __WASI_ENODEV: __wasi_errno_t = 43;
pub const __WASI_ENOENT: __wasi_errno_t = 44;
pub const __WASI_ENOEXEC: __wasi_errno_t = 45;
pub const __WASI_ENOLCK: __wasi_errno_t = 46;
pub const __WASI_ENOLINK: __wasi_errno_t = 47;
pub const __WASI_ENOMEM: __wasi_errno_t = 48;
pub const __WASI_ENOMSG: __wasi_errno_t = 49;
pub const __WASI_ENOPROTOOPT: __wasi_errno_t = 50;
pub const __WASI_ENOSPC: __wasi_errno_t = 51;
pub const __WASI_ENOSYS: __wasi_errno_t = 52;
pub const __WASI_ENOTCONN: __wasi_errno_t = 53;
pub const __WASI_ENOTDIR: __wasi_errno_t = 54;
pub const __WASI_ENOTEMPTY: __wasi_errno_t = 55;
pub const __WASI_ENOTRECOVERABLE: __wasi_errno_t = 56;
pub const __WASI_ENOTSOCK: __wasi_errno_t = 57;
pub const __WASI_ENOTSUP: __wasi_errno_t = 58;
pub const __WASI_ENOTTY: __wasi_errno_t = 59;
pub const __WASI_ENXIO: __wasi_errno_t = 60;
pub const __WASI_EOVERFLOW: __wasi_errno_t = 61;
pub const __WASI_EOWNERDEAD: __wasi_errno_t = 62;
pub const __WASI_EPERM: __wasi_errno_t = 63;
pub const __WASI_EPIPE: __wasi_errno_t = 64;
pub const __WASI_EPROTO: __wasi_errno_t = 65;
pub const __WASI_EPROTONOSUPPORT: __wasi_errno_t = 66;
pub const __WASI_EPROTOTYPE: __wasi_errno_t = 67;
pub const __WASI_ERANGE: __wasi_errno_t = 68;
pub const __WASI_EROFS: __wasi_errno_t = 69;
pub const __WASI_ESPIPE: __wasi_errno_t = 70;
pub const __WASI_ESRCH: __wasi_errno_t = 71;
pub const __WASI_ESTALE: __wasi_errno_t = 72;
pub const __WASI_ETIMEDOUT: __wasi_errno_t = 73;
pub const __WASI_ETXTBSY: __wasi_errno_t = 74;
pub const __WASI_EXDEV: __wasi_errno_t = 75;
pub const __WASI_ENOTCAPABLE: __wasi_errno_t = 76;

pub type __wasi_eventrwflags_t = u16;
pub const __WASI_EVENT_FD_READWRITE_HANGUP: __wasi_eventrwflags_t = 0x0001;

pub type __wasi_eventtype_t = u8;
pub const __WASI_EVENTTYPE_CLOCK: __wasi_eventtype_t = 0;
pub const __WASI_EVENTTYPE_FD_READ: __wasi_eventtype_t = 1;
pub const __WASI_EVENTTYPE_FD_WRITE: __wasi_eventtype_t = 2;

pub type __wasi_exitcode_t = u32;

pub type __wasi_fd_t = u32;

pub type __wasi_fdflags_t = u16;
pub const __WASI_FDFLAG_APPEND: __wasi_fdflags_t = 0x0001;
pub const __WASI_FDFLAG_DSYNC: __wasi_fdflags_t = 0x0002;
pub const __WASI_FDFLAG_NONBLOCK: __wasi_fdflags_t = 0x0004;
pub const __WASI_FDFLAG_RSYNC: __wasi_fdflags_t = 0x0008;
pub const __WASI_FDFLAG_SYNC: __wasi_fdflags_t = 0x0010;

pub type __wasi_filedelta_t = i64;

pub type __wasi_filesize_t = u64;

pub type __wasi_filetype_t = u8;
pub const __WASI_FILETYPE_UNKNOWN: __wasi_filetype_t = 0;
pub const __WASI_FILETYPE_BLOCK_DEVICE: __wasi_filetype_t = 1;
pub const __WASI_FILETYPE_CHARACTER_DEVICE: __wasi_filetype_t = 2;
pub const __WASI_FILETYPE_DIRECTORY: __wasi_filetype_t = 3;
pub const __WASI_FILETYPE_REGULAR_FILE: __wasi_filetype_t = 4;
pub const __WASI_FILETYPE_SOCKET_DGRAM: __wasi_filetype_t = 5;
pub const __WASI_FILETYPE_SOCKET_STREAM: __wasi_filetype_t = 6;
pub const __WASI_FILETYPE_SYMBOLIC_LINK: __wasi_filetype_t = 7;

pub type __wasi_fstflags_t = u16;
pub const __WASI_FILESTAT_SET_ATIM: __wasi_fstflags_t = 0x0001;
pub const __WASI_FILESTAT_SET_ATIM_NOW: __wasi_fstflags_t = 0x0002;
pub const __WASI_FILESTAT_SET_MTIM: __wasi_fstflags_t = 0x0004;
pub const __WASI_FILESTAT_SET_MTIM_NOW: __wasi_fstflags_t = 0x0008;

pub type __wasi_inode_t = u64;

pub type __wasi_linkcount_t = u32;

pub type __wasi_lookupflags_t = u32;
pub const __WASI_LOOKUP_SYMLINK_FOLLOW: __wasi_lookupflags_t = 0x00000001;

pub type __wasi_oflags_t = u16;
pub const __WASI_O_CREAT: __wasi_oflags_t = 0x0001;
pub const __WASI_O_DIRECTORY: __wasi_oflags_t = 0x0002;
pub const __WASI_O_EXCL: __wasi_oflags_t = 0x0004;
pub const __WASI_O_TRUNC: __wasi_oflags_t = 0x0008;

pub type __wasi_riflags_t = u16;
pub const __WASI_SOCK_RECV_PEEK: __wasi_riflags_t = 0x0001;
pub const __WASI_SOCK_RECV_WAITALL: __wasi_riflags_t = 0x0002;

pub type __wasi_rights_t = u64;
pub const __WASI_RIGHT_FD_DATASYNC: __wasi_rights_t = 0x0000000000000001;
pub const __WASI_RIGHT_FD_READ: __wasi_rights_t = 0x0000000000000002;
pub const __WASI_RIGHT_FD_SEEK: __wasi_rights_t = 0x0000000000000004;
pub const __WASI_RIGHT_FD_FDSTAT_SET_FLAGS: __wasi_rights_t = 0x0000000000000008;
pub const __WASI_RIGHT_FD_SYNC: __wasi_rights_t = 0x0000000000000010;
pub const __WASI_RIGHT_FD_TELL: __wasi_rights_t = 0x0000000000000020;
pub const __WASI_RIGHT_FD_WRITE: __wasi_rights_t = 0x0000000000000040;
pub const __WASI_RIGHT_FD_ADVISE: __wasi_rights_t = 0x0000000000000080;
pub const __WASI_RIGHT_FD_ALLOCATE: __wasi_rights_t = 0x0000000000000100;
pub const __WASI_RIGHT_PATH_CREATE_DIRECTORY: __wasi_rights_t = 0x0000000000000200;
pub const __WASI_RIGHT_PATH_CREATE_FILE: __wasi_rights_t = 0x0000000000000400;
pub const __WASI_RIGHT_PATH_LINK_SOURCE: __wasi_rights_t = 0x0000000000000800;
pub const __WASI_RIGHT_PATH_LINK_TARGET: __wasi_rights_t = 0x0000000000001000;
pub const __WASI_RIGHT_PATH_OPEN: __wasi_rights_t = 0x0000000000002000;
pub const __WASI_RIGHT_FD_READDIR: __wasi_rights_t = 0x0000000000004000;
pub const __WASI_RIGHT_PATH_READLINK: __wasi_rights_t = 0x0000000000008000;
pub const __WASI_RIGHT_PATH_RENAME_SOURCE: __wasi_rights_t = 0x0000000000010000;
pub const __WASI_RIGHT_PATH_RENAME_TARGET: __wasi_rights_t = 0x0000000000020000;
pub const __WASI_RIGHT_PATH_FILESTAT_GET: __wasi_rights_t = 0x0000000000040000;
pub const __WASI_RIGHT_PATH_FILESTAT_SET_SIZE: __wasi_rights_t = 0x0000000000080000;
pub const __WASI_RIGHT_PATH_FILESTAT_SET_TIMES: __wasi_rights_t = 0x0000000000100000;
pub const __WASI_RIGHT_FD_FILESTAT_GET: __wasi_rights_t = 0x0000000000200000;
pub const __WASI_RIGHT_FD_FILESTAT_SET_SIZE: __wasi_rights_t = 0x0000000000400000;
pub const __WASI_RIGHT_FD_FILESTAT_SET_TIMES: __wasi_rights_t = 0x0000000000800000;
pub const __WASI_RIGHT_PATH_SYMLINK: __wasi_rights_t = 0x0000000001000000;
pub const __WASI_RIGHT_PATH_REMOVE_DIRECTORY: __wasi_rights_t = 0x0000000002000000;
pub const __WASI_RIGHT_PATH_UNLINK_FILE: __wasi_rights_t = 0x0000000004000000;
pub const __WASI_RIGHT_POLL_FD_READWRITE: __wasi_rights_t = 0x0000000008000000;
pub const __WASI_RIGHT_SOCK_SHUTDOWN: __wasi_rights_t = 0x0000000010000000;

pub const RIGHTS_ALL: __wasi_rights_t = __WASI_RIGHT_FD_DATASYNC
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
    | __WASI_RIGHT_SOCK_SHUTDOWN;

// Block and character device interaction is outside the scope of
// WASI. Simply allow everything.
pub const RIGHTS_BLOCK_DEVICE_BASE: __wasi_rights_t = RIGHTS_ALL;
pub const RIGHTS_BLOCK_DEVICE_INHERITING: __wasi_rights_t = RIGHTS_ALL;
pub const RIGHTS_CHARACTER_DEVICE_BASE: __wasi_rights_t = RIGHTS_ALL;
pub const RIGHTS_CHARACTER_DEVICE_INHERITING: __wasi_rights_t = RIGHTS_ALL;

// Only allow directory operations on directories. Directories can only
// yield file descriptors to other directories and files.
pub const RIGHTS_DIRECTORY_BASE: __wasi_rights_t = __WASI_RIGHT_FD_FDSTAT_SET_FLAGS
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
    | __WASI_RIGHT_FD_FILESTAT_SET_TIMES
    | __WASI_RIGHT_PATH_SYMLINK
    | __WASI_RIGHT_PATH_UNLINK_FILE
    | __WASI_RIGHT_PATH_REMOVE_DIRECTORY
    | __WASI_RIGHT_POLL_FD_READWRITE;
pub const RIGHTS_DIRECTORY_INHERITING: __wasi_rights_t =
    RIGHTS_DIRECTORY_BASE | RIGHTS_REGULAR_FILE_BASE;

// Operations that apply to regular files.
pub const RIGHTS_REGULAR_FILE_BASE: __wasi_rights_t = __WASI_RIGHT_FD_DATASYNC
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
    | __WASI_RIGHT_POLL_FD_READWRITE;
pub const RIGHTS_REGULAR_FILE_INHERITING: __wasi_rights_t = 0;

// Operations that apply to shared memory objects.
pub const RIGHTS_SHARED_MEMORY_BASE: __wasi_rights_t = __WASI_RIGHT_FD_READ
    | __WASI_RIGHT_FD_WRITE
    | __WASI_RIGHT_FD_FILESTAT_GET
    | __WASI_RIGHT_FD_FILESTAT_SET_SIZE;
pub const RIGHTS_SHARED_MEMORY_INHERITING: __wasi_rights_t = 0;

// Operations that apply to sockets and socket pairs.
pub const RIGHTS_SOCKET_BASE: __wasi_rights_t = __WASI_RIGHT_FD_READ
    | __WASI_RIGHT_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHT_FD_WRITE
    | __WASI_RIGHT_FD_FILESTAT_GET
    | __WASI_RIGHT_POLL_FD_READWRITE
    | __WASI_RIGHT_SOCK_SHUTDOWN;
pub const RIGHTS_SOCKET_INHERITING: __wasi_rights_t = RIGHTS_ALL;

// Operations that apply to TTYs.
pub const RIGHTS_TTY_BASE: __wasi_rights_t = __WASI_RIGHT_FD_READ
    | __WASI_RIGHT_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHT_FD_WRITE
    | __WASI_RIGHT_FD_FILESTAT_GET
    | __WASI_RIGHT_POLL_FD_READWRITE;
pub const RIGHTS_TTY_INHERITING: __wasi_rights_t = 0;

pub type __wasi_roflags_t = u16;
pub const __WASI_SOCK_RECV_DATA_TRUNCATED: __wasi_roflags_t = 0x0001;

pub type __wasi_sdflags_t = u8;
pub const __WASI_SHUT_RD: __wasi_sdflags_t = 0x01;
pub const __WASI_SHUT_WR: __wasi_sdflags_t = 0x02;

pub type __wasi_siflags_t = u16;

pub type __wasi_signal_t = u8;
// 0 is reserved; POSIX has special semantics for kill(pid, 0).
pub const __WASI_SIGHUP: __wasi_signal_t = 1;
pub const __WASI_SIGINT: __wasi_signal_t = 2;
pub const __WASI_SIGQUIT: __wasi_signal_t = 3;
pub const __WASI_SIGILL: __wasi_signal_t = 4;
pub const __WASI_SIGTRAP: __wasi_signal_t = 5;
pub const __WASI_SIGABRT: __wasi_signal_t = 6;
pub const __WASI_SIGBUS: __wasi_signal_t = 7;
pub const __WASI_SIGFPE: __wasi_signal_t = 8;
pub const __WASI_SIGKILL: __wasi_signal_t = 9;
pub const __WASI_SIGUSR1: __wasi_signal_t = 10;
pub const __WASI_SIGSEGV: __wasi_signal_t = 11;
pub const __WASI_SIGUSR2: __wasi_signal_t = 12;
pub const __WASI_SIGPIPE: __wasi_signal_t = 13;
pub const __WASI_SIGALRM: __wasi_signal_t = 14;
pub const __WASI_SIGTERM: __wasi_signal_t = 15;
pub const __WASI_SIGCHLD: __wasi_signal_t = 16;
pub const __WASI_SIGCONT: __wasi_signal_t = 17;
pub const __WASI_SIGSTOP: __wasi_signal_t = 18;
pub const __WASI_SIGTSTP: __wasi_signal_t = 19;
pub const __WASI_SIGTTIN: __wasi_signal_t = 20;
pub const __WASI_SIGTTOU: __wasi_signal_t = 21;
pub const __WASI_SIGURG: __wasi_signal_t = 22;
pub const __WASI_SIGXCPU: __wasi_signal_t = 23;
pub const __WASI_SIGXFSZ: __wasi_signal_t = 24;
pub const __WASI_SIGVTALRM: __wasi_signal_t = 25;
pub const __WASI_SIGPROF: __wasi_signal_t = 26;
pub const __WASI_SIGWINCH: __wasi_signal_t = 27;
pub const __WASI_SIGPOLL: __wasi_signal_t = 28;
pub const __WASI_SIGPWR: __wasi_signal_t = 29;
pub const __WASI_SIGSYS: __wasi_signal_t = 30;

pub type __wasi_subclockflags_t = u16;
pub const __WASI_SUBSCRIPTION_CLOCK_ABSTIME: __wasi_subclockflags_t = 0x0001;

pub type __wasi_timestamp_t = u64;

pub type __wasi_userdata_t = u64;

pub type __wasi_whence_t = u8;
pub const __WASI_WHENCE_CUR: __wasi_whence_t = 0;
pub const __WASI_WHENCE_END: __wasi_whence_t = 1;
pub const __WASI_WHENCE_SET: __wasi_whence_t = 2;

pub type __wasi_preopentype_t = u8;
pub const __WASI_PREOPENTYPE_DIR: __wasi_preopentype_t = 0;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_dirent_t {
    pub d_next: __wasi_dircookie_t,
    pub d_ino: __wasi_inode_t,
    pub d_namlen: u32,
    pub d_type: __wasi_filetype_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct __wasi_event_t {
    pub userdata: __wasi_userdata_t,
    pub error: __wasi_errno_t,
    pub type_: __wasi_eventtype_t,
    pub u: __wasi_event_t___wasi_event_u,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union __wasi_event_t___wasi_event_u {
    pub fd_readwrite: __wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t,
    _bindgen_union_align: [u64; 2usize],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
    pub nbytes: __wasi_filesize_t,
    pub flags: __wasi_eventrwflags_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct __wasi_prestat_t {
    pub pr_type: __wasi_preopentype_t,
    pub u: __wasi_prestat_t___wasi_prestat_u,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union __wasi_prestat_t___wasi_prestat_u {
    pub dir: __wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t,
    _bindgen_union_align: u64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t {
    pub pr_name_len: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_fdstat_t {
    pub fs_filetype: __wasi_filetype_t,
    pub fs_flags: __wasi_fdflags_t,
    pub fs_rights_base: __wasi_rights_t,
    pub fs_rights_inheriting: __wasi_rights_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_filestat_t {
    pub st_dev: __wasi_device_t,
    pub st_ino: __wasi_inode_t,
    pub st_filetype: __wasi_filetype_t,
    pub st_nlink: __wasi_linkcount_t,
    pub st_size: __wasi_filesize_t,
    pub st_atim: __wasi_timestamp_t,
    pub st_mtim: __wasi_timestamp_t,
    pub st_ctim: __wasi_timestamp_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_ciovec_t {
    pub buf: *const void,
    pub buf_len: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_iovec_t {
    pub buf: *mut void,
    pub buf_len: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct __wasi_subscription_t {
    pub userdata: __wasi_userdata_t,
    pub type_: __wasi_eventtype_t,
    pub u: __wasi_subscription_t___wasi_subscription_u,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union __wasi_subscription_t___wasi_subscription_u {
    pub clock: __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
    pub fd_readwrite:
        __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_fd_readwrite_t,
    _bindgen_union_align: [u64; 5usize],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t {
    pub identifier: __wasi_userdata_t,
    pub clock_id: __wasi_clockid_t,
    pub timeout: __wasi_timestamp_t,
    pub precision: __wasi_timestamp_t,
    pub flags: __wasi_subclockflags_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_fd_readwrite_t {
    pub fd: __wasi_fd_t,
}

pub unsafe fn ciovec_to_host<'a>(ciovec: &'a __wasi_ciovec_t) -> io::IoSlice<'a> {
    let slice = slice::from_raw_parts(ciovec.buf as *const u8, ciovec.buf_len);
    io::IoSlice::new(slice)
}

pub unsafe fn ciovec_to_host_mut<'a>(ciovec: &'a mut __wasi_ciovec_t) -> io::IoSliceMut<'a> {
    let slice = slice::from_raw_parts_mut(ciovec.buf as *mut u8, ciovec.buf_len);
    io::IoSliceMut::new(slice)
}

pub unsafe fn iovec_to_host<'a>(iovec: &'a __wasi_iovec_t) -> io::IoSlice<'a> {
    let slice = slice::from_raw_parts(iovec.buf as *const u8, iovec.buf_len);
    io::IoSlice::new(slice)
}

pub unsafe fn iovec_to_host_mut<'a>(iovec: &'a mut __wasi_iovec_t) -> io::IoSliceMut<'a> {
    let slice = slice::from_raw_parts_mut(iovec.buf as *mut u8, iovec.buf_len);
    io::IoSliceMut::new(slice)
}

/// Creates not-owned WASI path from byte slice.
///
/// NB WASI spec requires bytes to be valid UTF-8. Otherwise,
/// `__WASI_EILSEQ` error is returned.
pub fn path_from_slice<'a>(s: &'a [u8]) -> Result<&'a str> {
    str::from_utf8(s).map_err(|_| __WASI_EILSEQ)
}

/// Creates owned WASI path from byte vector.
///
/// NB WASI spec requires bytes to be valid UTF-8. Otherwise,
/// `__WASI_EILSEQ` error is returned.
pub fn path_from_vec<S: Into<Vec<u8>>>(s: S) -> Result<String> {
    String::from_utf8(s.into()).map_err(|_| __WASI_EILSEQ)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindgen_test_layout___wasi_dirent_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_dirent_t>(),
            24usize,
            concat!("Size of: ", stringify!(__wasi_dirent_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_dirent_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_dirent_t))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_dirent_t>())).d_next as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_dirent_t),
                "::",
                stringify!(d_next)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_dirent_t>())).d_ino as *const _ as usize },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_dirent_t),
                "::",
                stringify!(d_ino)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_dirent_t>())).d_namlen as *const _ as usize },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_dirent_t),
                "::",
                stringify!(d_namlen)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_dirent_t>())).d_type as *const _ as usize },
            20usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_dirent_t),
                "::",
                stringify!(d_type)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_fd_readwrite_t(
    ) {
        assert_eq!(
            ::std::mem::size_of::<
                __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_fd_readwrite_t,
            >(),
            4usize,
            concat!(
                "Size of: ",
                stringify!(
                __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_fd_readwrite_t
            )
            )
        );
        assert_eq!(
            ::std::mem::align_of::<
                __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_fd_readwrite_t,
            >(),
            4usize,
            concat!(
                "Alignment of ",
                stringify!(
                __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_fd_readwrite_t
            )
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<
                __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_fd_readwrite_t,
            >()))
            .fd as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(
                __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_fd_readwrite_t
            ),
                "::",
                stringify!(fd)
            )
        );
    }
    #[test]
    fn bindgen_test_layout___wasi_subscription_t___wasi_subscription_u() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_subscription_t___wasi_subscription_u>(),
            40usize,
            concat!(
                "Size of: ",
                stringify!(__wasi_subscription_t___wasi_subscription_u)
            )
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_subscription_t___wasi_subscription_u>(),
            8usize,
            concat!(
                "Alignment of ",
                stringify!(__wasi_subscription_t___wasi_subscription_u)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_subscription_t___wasi_subscription_u>())).clock
                    as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_t___wasi_subscription_u),
                "::",
                stringify!(clock)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_subscription_t___wasi_subscription_u>())).fd_readwrite
                    as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_t___wasi_subscription_u),
                "::",
                stringify!(fd_readwrite)
            )
        );
    }
    #[test]
    fn bindgen_test_layout___wasi_subscription_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_subscription_t>(),
            56usize,
            concat!("Size of: ", stringify!(__wasi_subscription_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_subscription_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_subscription_t))
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_subscription_t>())).userdata as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_t),
                "::",
                stringify!(userdata)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_subscription_t>())).type_ as *const _ as usize },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_t),
                "::",
                stringify!(type_)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_subscription_t>())).u as *const _ as usize },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_t),
                "::",
                stringify!(u)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t(
    ) {
        assert_eq!(
            ::std::mem::size_of::<
                __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
            >(),
            40usize,
            concat!(
                "Size of: ",
                stringify!(
                    __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t
                )
            )
        );
        assert_eq!(
            ::std::mem::align_of::<
                __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
            >(),
            8usize,
            concat!(
                "Alignment of ",
                stringify!(
                    __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t
                )
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<
                    __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
                >()))
                .identifier as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(
                    __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t
                ),
                "::",
                stringify!(identifier)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<
                    __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
                >()))
                .clock_id as *const _ as usize
            },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(
                    __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t
                ),
                "::",
                stringify!(clock_id)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<
                    __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
                >()))
                .timeout as *const _ as usize
            },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(
                    __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t
                ),
                "::",
                stringify!(timeout)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<
                    __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
                >()))
                .precision as *const _ as usize
            },
            24usize,
            concat!(
                "Offset of field: ",
                stringify!(
                    __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t
                ),
                "::",
                stringify!(precision)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<
                    __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
                >()))
                .flags as *const _ as usize
            },
            32usize,
            concat!(
                "Offset of field: ",
                stringify!(
                    __wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t
                ),
                "::",
                stringify!(flags)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_iovec_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_iovec_t>(),
            16usize,
            concat!("Size of: ", stringify!(__wasi_iovec_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_iovec_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_iovec_t))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_iovec_t>())).buf as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_iovec_t),
                "::",
                stringify!(buf)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_iovec_t>())).buf_len as *const _ as usize },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_iovec_t),
                "::",
                stringify!(buf_len)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_ciovec_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_ciovec_t>(),
            16usize,
            concat!("Size of: ", stringify!(__wasi_ciovec_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_ciovec_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_ciovec_t))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_ciovec_t>())).buf as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_ciovec_t),
                "::",
                stringify!(buf)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_ciovec_t>())).buf_len as *const _ as usize },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_ciovec_t),
                "::",
                stringify!(buf_len)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_filestat_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_filestat_t>(),
            56usize,
            concat!("Size of: ", stringify!(__wasi_filestat_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_filestat_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_filestat_t))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).st_dev as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_dev)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).st_ino as *const _ as usize },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_ino)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_filestat_t>())).st_filetype as *const _ as usize
            },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_filetype)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).st_nlink as *const _ as usize },
            20usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_nlink)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).st_size as *const _ as usize },
            24usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_size)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).st_atim as *const _ as usize },
            32usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_atim)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).st_mtim as *const _ as usize },
            40usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_mtim)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).st_ctim as *const _ as usize },
            48usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_ctim)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_fdstat_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_fdstat_t>(),
            24usize,
            concat!("Size of: ", stringify!(__wasi_fdstat_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_fdstat_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_fdstat_t))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_fdstat_t>())).fs_filetype as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_fdstat_t),
                "::",
                stringify!(fs_filetype)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_fdstat_t>())).fs_flags as *const _ as usize },
            2usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_fdstat_t),
                "::",
                stringify!(fs_flags)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_fdstat_t>())).fs_rights_base as *const _ as usize
            },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_fdstat_t),
                "::",
                stringify!(fs_rights_base)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_fdstat_t>())).fs_rights_inheriting as *const _
                    as usize
            },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_fdstat_t),
                "::",
                stringify!(fs_rights_inheriting)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_prestat_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_prestat_t>(),
            16usize,
            concat!("Size of: ", stringify!(__wasi_prestat_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_prestat_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_prestat_t))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_prestat_t>())).pr_type as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_prestat_t),
                "::",
                stringify!(pr_type)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_prestat_t>())).u as *const _ as usize },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_prestat_t),
                "::",
                stringify!(u)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t>(),
            8usize,
            concat!(
                "Size of: ",
                stringify!(__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t)
            )
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t>(),
            8usize,
            concat!(
                "Alignment of ",
                stringify!(__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t>()))
                    .pr_name_len as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t),
                "::",
                stringify!(pr_name_len)
            )
        );
    }
    #[test]
    fn bindgen_test_layout___wasi_prestat_t___wasi_prestat_u() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_prestat_t___wasi_prestat_u>(),
            8usize,
            concat!("Size of: ", stringify!(__wasi_prestat_t___wasi_prestat_u))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_prestat_t___wasi_prestat_u>(),
            8usize,
            concat!(
                "Alignment of ",
                stringify!(__wasi_prestat_t___wasi_prestat_u)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_prestat_t___wasi_prestat_u>())).dir as *const _
                    as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_prestat_t___wasi_prestat_u),
                "::",
                stringify!(dir)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t>(),
            16usize,
            concat!(
                "Size of: ",
                stringify!(__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t)
            )
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t>(),
            8usize,
            concat!(
                "Alignment of ",
                stringify!(__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t>(
                )))
                .nbytes as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t),
                "::",
                stringify!(nbytes)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t>(
                )))
                .flags as *const _ as usize
            },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t),
                "::",
                stringify!(flags)
            )
        );
    }
    #[test]
    fn bindgen_test_layout___wasi_event_t___wasi_event_u() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_event_t___wasi_event_u>(),
            16usize,
            concat!("Size of: ", stringify!(__wasi_event_t___wasi_event_u))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_event_t___wasi_event_u>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_event_t___wasi_event_u))
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_event_t___wasi_event_u>())).fd_readwrite as *const _
                    as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_event_t___wasi_event_u),
                "::",
                stringify!(fd_readwrite)
            )
        );
    }
    #[test]
    fn bindgen_test_layout___wasi_event_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_event_t>(),
            32usize,
            concat!("Size of: ", stringify!(__wasi_event_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_event_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_event_t))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_event_t>())).userdata as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_event_t),
                "::",
                stringify!(userdata)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_event_t>())).error as *const _ as usize },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_event_t),
                "::",
                stringify!(error)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_event_t>())).type_ as *const _ as usize },
            10usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_event_t),
                "::",
                stringify!(type_)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_event_t>())).u as *const _ as usize },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_event_t),
                "::",
                stringify!(u)
            )
        );
    }
}
