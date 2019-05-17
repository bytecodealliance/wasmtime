//! WASI host types as defined in host. This file was originally generated
//! by running bindgen over wasi/core.h, and the content
//! still largely reflects that.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

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
    | __WASI_RIGHT_FD_FILESTAT_SET_SIZE
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

pub fn errno_from_nix(errno: nix::errno::Errno) -> __wasi_errno_t {
    match errno {
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
    }
}

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

pub unsafe fn iovec_to_nix<'a>(iovec: &'a __wasi_iovec_t) -> nix::sys::uio::IoVec<&'a [u8]> {
    let slice = std::slice::from_raw_parts(iovec.buf as *const u8, iovec.buf_len);
    nix::sys::uio::IoVec::from_slice(slice)
}

pub unsafe fn iovec_to_nix_mut<'a>(
    iovec: &'a mut __wasi_iovec_t,
) -> nix::sys::uio::IoVec<&'a mut [u8]> {
    let slice = std::slice::from_raw_parts_mut(iovec.buf as *mut u8, iovec.buf_len);
    nix::sys::uio::IoVec::from_mut_slice(slice)
}

#[cfg(target_os = "linux")]
pub const O_RSYNC: nix::fcntl::OFlag = nix::fcntl::OFlag::O_RSYNC;

#[cfg(not(target_os = "linux"))]
pub const O_RSYNC: nix::fcntl::OFlag = nix::fcntl::OFlag::O_SYNC;

pub fn nix_from_fdflags(fdflags: __wasi_fdflags_t) -> nix::fcntl::OFlag {
    use nix::fcntl::OFlag;
    let mut nix_flags = OFlag::empty();
    if fdflags & __WASI_FDFLAG_APPEND != 0 {
        nix_flags.insert(OFlag::O_APPEND);
    }
    if fdflags & __WASI_FDFLAG_DSYNC != 0 {
        nix_flags.insert(OFlag::O_DSYNC);
    }
    if fdflags & __WASI_FDFLAG_NONBLOCK != 0 {
        nix_flags.insert(OFlag::O_NONBLOCK);
    }
    if fdflags & __WASI_FDFLAG_RSYNC != 0 {
        nix_flags.insert(O_RSYNC);
    }
    if fdflags & __WASI_FDFLAG_SYNC != 0 {
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
    fdflags
}

pub fn nix_from_oflags(oflags: __wasi_oflags_t) -> nix::fcntl::OFlag {
    use nix::fcntl::OFlag;
    let mut nix_flags = OFlag::empty();
    if oflags & __WASI_O_CREAT != 0 {
        nix_flags.insert(OFlag::O_CREAT);
    }
    if oflags & __WASI_O_DIRECTORY != 0 {
        nix_flags.insert(OFlag::O_DIRECTORY);
    }
    if oflags & __WASI_O_EXCL != 0 {
        nix_flags.insert(OFlag::O_EXCL);
    }
    if oflags & __WASI_O_TRUNC != 0 {
        nix_flags.insert(OFlag::O_TRUNC);
    }
    nix_flags
}

pub fn filetype_from_nix(sflags: nix::sys::stat::SFlag) -> __wasi_filetype_t {
    use nix::sys::stat::SFlag;
    if sflags.contains(SFlag::S_IFCHR) {
        __WASI_FILETYPE_CHARACTER_DEVICE
    } else if sflags.contains(SFlag::S_IFBLK) {
        __WASI_FILETYPE_BLOCK_DEVICE
    } else if sflags.contains(SFlag::S_IFIFO) | sflags.contains(SFlag::S_IFSOCK) {
        __WASI_FILETYPE_SOCKET_STREAM
    } else if sflags.contains(SFlag::S_IFDIR) {
        __WASI_FILETYPE_DIRECTORY
    } else if sflags.contains(SFlag::S_IFREG) {
        __WASI_FILETYPE_REGULAR_FILE
    } else if sflags.contains(SFlag::S_IFLNK) {
        __WASI_FILETYPE_SYMBOLIC_LINK
    } else {
        __WASI_FILETYPE_UNKNOWN
    }
}

pub fn nix_from_filetype(sflags: __wasi_filetype_t) -> nix::sys::stat::SFlag {
    use nix::sys::stat::SFlag;
    let mut nix_sflags = SFlag::empty();
    if sflags & __WASI_FILETYPE_CHARACTER_DEVICE != 0 {
        nix_sflags.insert(SFlag::S_IFCHR);
    }
    if sflags & __WASI_FILETYPE_BLOCK_DEVICE != 0 {
        nix_sflags.insert(SFlag::S_IFBLK);
    }
    if sflags & __WASI_FILETYPE_SOCKET_STREAM != 0 {
        nix_sflags.insert(SFlag::S_IFIFO);
        nix_sflags.insert(SFlag::S_IFSOCK);
    }
    if sflags & __WASI_FILETYPE_DIRECTORY != 0 {
        nix_sflags.insert(SFlag::S_IFDIR);
    }
    if sflags & __WASI_FILETYPE_REGULAR_FILE != 0 {
        nix_sflags.insert(SFlag::S_IFREG);
    }
    if sflags & __WASI_FILETYPE_SYMBOLIC_LINK != 0 {
        nix_sflags.insert(SFlag::S_IFLNK);
    }
    nix_sflags
}

pub fn filestat_from_nix(filestat: nix::sys::stat::FileStat) -> __wasi_filestat_t {
    use std::convert::TryFrom;

    let filetype = nix::sys::stat::SFlag::from_bits_truncate(filestat.st_mode);
    let dev = __wasi_device_t::try_from(filestat.st_dev)
        .expect("FileStat::st_dev is trivially convertible to __wasi_device_t");
    let ino = __wasi_inode_t::try_from(filestat.st_ino)
        .expect("FileStat::st_ino is trivially convertible to __wasi_inode_t");

    __wasi_filestat_t {
        st_dev: dev,
        st_ino: ino,
        st_nlink: filestat.st_nlink as __wasi_linkcount_t,
        st_size: filestat.st_size as __wasi_filesize_t,
        st_atim: filestat.st_atime as __wasi_timestamp_t,
        st_ctim: filestat.st_ctime as __wasi_timestamp_t,
        st_mtim: filestat.st_mtime as __wasi_timestamp_t,
        st_filetype: filetype_from_nix(filetype),
    }
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
