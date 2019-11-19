//! Types and constants shared between 32-bit and 64-bit wasi. Types involving
//! pointer or `usize`-sized data are excluded here, so this file only contains
//! fixed-size types, so it's host/target independent.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use wig::witx_wasi_types;

witx_wasi_types!("snapshot" "wasi_snapshot_preview1");

pub(crate) const RIGHTS_ALL: __wasi_rights_t = __WASI_RIGHTS_FD_DATASYNC
    | __WASI_RIGHTS_FD_READ
    | __WASI_RIGHTS_FD_SEEK
    | __WASI_RIGHTS_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHTS_FD_SYNC
    | __WASI_RIGHTS_FD_TELL
    | __WASI_RIGHTS_FD_WRITE
    | __WASI_RIGHTS_FD_ADVISE
    | __WASI_RIGHTS_FD_ALLOCATE
    | __WASI_RIGHTS_PATH_CREATE_DIRECTORY
    | __WASI_RIGHTS_PATH_CREATE_FILE
    | __WASI_RIGHTS_PATH_LINK_SOURCE
    | __WASI_RIGHTS_PATH_LINK_TARGET
    | __WASI_RIGHTS_PATH_OPEN
    | __WASI_RIGHTS_FD_READDIR
    | __WASI_RIGHTS_PATH_READLINK
    | __WASI_RIGHTS_PATH_RENAME_SOURCE
    | __WASI_RIGHTS_PATH_RENAME_TARGET
    | __WASI_RIGHTS_PATH_FILESTAT_GET
    | __WASI_RIGHTS_PATH_FILESTAT_SET_SIZE
    | __WASI_RIGHTS_PATH_FILESTAT_SET_TIMES
    | __WASI_RIGHTS_FD_FILESTAT_GET
    | __WASI_RIGHTS_FD_FILESTAT_SET_SIZE
    | __WASI_RIGHTS_FD_FILESTAT_SET_TIMES
    | __WASI_RIGHTS_PATH_SYMLINK
    | __WASI_RIGHTS_PATH_UNLINK_FILE
    | __WASI_RIGHTS_PATH_REMOVE_DIRECTORY
    | __WASI_RIGHTS_POLL_FD_READWRITE
    | __WASI_RIGHTS_SOCK_SHUTDOWN;

// Block and character device interaction is outside the scope of
// WASI. Simply allow everything.
pub(crate) const RIGHTS_BLOCK_DEVICE_BASE: __wasi_rights_t = RIGHTS_ALL;
pub(crate) const RIGHTS_BLOCK_DEVICE_INHERITING: __wasi_rights_t = RIGHTS_ALL;
pub(crate) const RIGHTS_CHARACTER_DEVICE_BASE: __wasi_rights_t = RIGHTS_ALL;
pub(crate) const RIGHTS_CHARACTER_DEVICE_INHERITING: __wasi_rights_t = RIGHTS_ALL;

// Only allow directory operations on directories. Directories can only
// yield file descriptors to other directories and files.
pub(crate) const RIGHTS_DIRECTORY_BASE: __wasi_rights_t = __WASI_RIGHTS_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHTS_FD_SYNC
    | __WASI_RIGHTS_FD_ADVISE
    | __WASI_RIGHTS_PATH_CREATE_DIRECTORY
    | __WASI_RIGHTS_PATH_CREATE_FILE
    | __WASI_RIGHTS_PATH_LINK_SOURCE
    | __WASI_RIGHTS_PATH_LINK_TARGET
    | __WASI_RIGHTS_PATH_OPEN
    | __WASI_RIGHTS_FD_READDIR
    | __WASI_RIGHTS_PATH_READLINK
    | __WASI_RIGHTS_PATH_RENAME_SOURCE
    | __WASI_RIGHTS_PATH_RENAME_TARGET
    | __WASI_RIGHTS_PATH_FILESTAT_GET
    | __WASI_RIGHTS_PATH_FILESTAT_SET_SIZE
    | __WASI_RIGHTS_PATH_FILESTAT_SET_TIMES
    | __WASI_RIGHTS_FD_FILESTAT_GET
    | __WASI_RIGHTS_FD_FILESTAT_SET_TIMES
    | __WASI_RIGHTS_PATH_SYMLINK
    | __WASI_RIGHTS_PATH_UNLINK_FILE
    | __WASI_RIGHTS_PATH_REMOVE_DIRECTORY
    | __WASI_RIGHTS_POLL_FD_READWRITE;
pub(crate) const RIGHTS_DIRECTORY_INHERITING: __wasi_rights_t =
    RIGHTS_DIRECTORY_BASE | RIGHTS_REGULAR_FILE_BASE;

// Operations that apply to regular files.
pub(crate) const RIGHTS_REGULAR_FILE_BASE: __wasi_rights_t = __WASI_RIGHTS_FD_DATASYNC
    | __WASI_RIGHTS_FD_READ
    | __WASI_RIGHTS_FD_SEEK
    | __WASI_RIGHTS_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHTS_FD_SYNC
    | __WASI_RIGHTS_FD_TELL
    | __WASI_RIGHTS_FD_WRITE
    | __WASI_RIGHTS_FD_ADVISE
    | __WASI_RIGHTS_FD_ALLOCATE
    | __WASI_RIGHTS_FD_FILESTAT_GET
    | __WASI_RIGHTS_FD_FILESTAT_SET_SIZE
    | __WASI_RIGHTS_FD_FILESTAT_SET_TIMES
    | __WASI_RIGHTS_POLL_FD_READWRITE;
pub(crate) const RIGHTS_REGULAR_FILE_INHERITING: __wasi_rights_t = 0;

// Operations that apply to sockets and socket pairs.
pub(crate) const RIGHTS_SOCKET_BASE: __wasi_rights_t = __WASI_RIGHTS_FD_READ
    | __WASI_RIGHTS_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHTS_FD_WRITE
    | __WASI_RIGHTS_FD_FILESTAT_GET
    | __WASI_RIGHTS_POLL_FD_READWRITE
    | __WASI_RIGHTS_SOCK_SHUTDOWN;
pub(crate) const RIGHTS_SOCKET_INHERITING: __wasi_rights_t = RIGHTS_ALL;

// Operations that apply to TTYs.
pub(crate) const RIGHTS_TTY_BASE: __wasi_rights_t = __WASI_RIGHTS_FD_READ
    | __WASI_RIGHTS_FD_FDSTAT_SET_FLAGS
    | __WASI_RIGHTS_FD_WRITE
    | __WASI_RIGHTS_FD_FILESTAT_GET
    | __WASI_RIGHTS_POLL_FD_READWRITE;
#[allow(unused)]
pub(crate) const RIGHTS_TTY_INHERITING: __wasi_rights_t = 0;

pub fn strerror(errno: __wasi_errno_t) -> &'static str {
    match errno {
        __WASI_ERRNO_SUCCESS => "__WASI_ERRNO_SUCCESS",
        __WASI_ERRNO_2BIG => "__WASI_ERRNO_2BIG",
        __WASI_ERRNO_ACCES => "__WASI_ERRNO_ACCES",
        __WASI_ERRNO_ADDRINUSE => "__WASI_ERRNO_ADDRINUSE",
        __WASI_ERRNO_ADDRNOTAVAIL => "__WASI_ERRNO_ADDRNOTAVAIL",
        __WASI_ERRNO_AFNOSUPPORT => "__WASI_ERRNO_AFNOSUPPORT",
        __WASI_ERRNO_AGAIN => "__WASI_ERRNO_AGAIN",
        __WASI_ERRNO_ALREADY => "__WASI_ERRNO_ALREADY",
        __WASI_ERRNO_BADF => "__WASI_ERRNO_BADF",
        __WASI_ERRNO_BADMSG => "__WASI_ERRNO_BADMSG",
        __WASI_ERRNO_BUSY => "__WASI_ERRNO_BUSY",
        __WASI_ERRNO_CANCELED => "__WASI_ERRNO_CANCELED",
        __WASI_ERRNO_CHILD => "__WASI_ERRNO_CHILD",
        __WASI_ERRNO_CONNABORTED => "__WASI_ERRNO_CONNABORTED",
        __WASI_ERRNO_CONNREFUSED => "__WASI_ERRNO_CONNREFUSED",
        __WASI_ERRNO_CONNRESET => "__WASI_ERRNO_CONNRESET",
        __WASI_ERRNO_DEADLK => "__WASI_ERRNO_DEADLK",
        __WASI_ERRNO_DESTADDRREQ => "__WASI_ERRNO_DESTADDRREQ",
        __WASI_ERRNO_DOM => "__WASI_ERRNO_DOM",
        __WASI_ERRNO_DQUOT => "__WASI_ERRNO_DQUOT",
        __WASI_ERRNO_EXIST => "__WASI_ERRNO_EXIST",
        __WASI_ERRNO_FAULT => "__WASI_ERRNO_FAULT",
        __WASI_ERRNO_FBIG => "__WASI_ERRNO_FBIG",
        __WASI_ERRNO_HOSTUNREACH => "__WASI_ERRNO_HOSTUNREACH",
        __WASI_ERRNO_IDRM => "__WASI_ERRNO_IDRM",
        __WASI_ERRNO_ILSEQ => "__WASI_ERRNO_ILSEQ",
        __WASI_ERRNO_INPROGRESS => "__WASI_ERRNO_INPROGRESS",
        __WASI_ERRNO_INTR => "__WASI_ERRNO_INTR",
        __WASI_ERRNO_INVAL => "__WASI_ERRNO_INVAL",
        __WASI_ERRNO_IO => "__WASI_ERRNO_IO",
        __WASI_ERRNO_ISCONN => "__WASI_ERRNO_ISCONN",
        __WASI_ERRNO_ISDIR => "__WASI_ERRNO_ISDIR",
        __WASI_ERRNO_LOOP => "__WASI_ERRNO_LOOP",
        __WASI_ERRNO_MFILE => "__WASI_ERRNO_MFILE",
        __WASI_ERRNO_MLINK => "__WASI_ERRNO_MLINK",
        __WASI_ERRNO_MSGSIZE => "__WASI_ERRNO_MSGSIZE",
        __WASI_ERRNO_MULTIHOP => "__WASI_ERRNO_MULTIHOP",
        __WASI_ERRNO_NAMETOOLONG => "__WASI_ERRNO_NAMETOOLONG",
        __WASI_ERRNO_NETDOWN => "__WASI_ERRNO_NETDOWN",
        __WASI_ERRNO_NETRESET => "__WASI_ERRNO_NETRESET",
        __WASI_ERRNO_NETUNREACH => "__WASI_ERRNO_NETUNREACH",
        __WASI_ERRNO_NFILE => "__WASI_ERRNO_NFILE",
        __WASI_ERRNO_NOBUFS => "__WASI_ERRNO_NOBUFS",
        __WASI_ERRNO_NODEV => "__WASI_ERRNO_NODEV",
        __WASI_ERRNO_NOENT => "__WASI_ERRNO_NOENT",
        __WASI_ERRNO_NOEXEC => "__WASI_ERRNO_NOEXEC",
        __WASI_ERRNO_NOLCK => "__WASI_ERRNO_NOLCK",
        __WASI_ERRNO_NOLINK => "__WASI_ERRNO_NOLINK",
        __WASI_ERRNO_NOMEM => "__WASI_ERRNO_NOMEM",
        __WASI_ERRNO_NOMSG => "__WASI_ERRNO_NOMSG",
        __WASI_ERRNO_NOPROTOOPT => "__WASI_ERRNO_NOPROTOOPT",
        __WASI_ERRNO_NOSPC => "__WASI_ERRNO_NOSPC",
        __WASI_ERRNO_NOSYS => "__WASI_ERRNO_NOSYS",
        __WASI_ERRNO_NOTCONN => "__WASI_ERRNO_NOTCONN",
        __WASI_ERRNO_NOTDIR => "__WASI_ERRNO_NOTDIR",
        __WASI_ERRNO_NOTEMPTY => "__WASI_ERRNO_NOTEMPTY",
        __WASI_ERRNO_NOTRECOVERABLE => "__WASI_ERRNO_NOTRECOVERABLE",
        __WASI_ERRNO_NOTSOCK => "__WASI_ERRNO_NOTSOCK",
        __WASI_ERRNO_NOTSUP => "__WASI_ERRNO_NOTSUP",
        __WASI_ERRNO_NOTTY => "__WASI_ERRNO_NOTTY",
        __WASI_ERRNO_NXIO => "__WASI_ERRNO_NXIO",
        __WASI_ERRNO_OVERFLOW => "__WASI_ERRNO_OVERFLOW",
        __WASI_ERRNO_OWNERDEAD => "__WASI_ERRNO_OWNERDEAD",
        __WASI_ERRNO_PERM => "__WASI_ERRNO_PERM",
        __WASI_ERRNO_PIPE => "__WASI_ERRNO_PIPE",
        __WASI_ERRNO_PROTO => "__WASI_ERRNO_PROTO",
        __WASI_ERRNO_PROTONOSUPPORT => "__WASI_ERRNO_PROTONOSUPPORT",
        __WASI_ERRNO_PROTOTYPE => "__WASI_ERRNO_PROTOTYPE",
        __WASI_ERRNO_RANGE => "__WASI_ERRNO_RANGE",
        __WASI_ERRNO_ROFS => "__WASI_ERRNO_ROFS",
        __WASI_ERRNO_SPIPE => "__WASI_ERRNO_SPIPE",
        __WASI_ERRNO_SRCH => "__WASI_ERRNO_SRCH",
        __WASI_ERRNO_STALE => "__WASI_ERRNO_STALE",
        __WASI_ERRNO_TIMEDOUT => "__WASI_ERRNO_TIMEDOUT",
        __WASI_ERRNO_TXTBSY => "__WASI_ERRNO_TXTBSY",
        __WASI_ERRNO_XDEV => "__WASI_ERRNO_XDEV",
        __WASI_ERRNO_NOTCAPABLE => "__WASI_ERRNO_NOTCAPABLE",
        other => panic!("Undefined errno value {:?}", other),
    }
}

pub fn whence_to_str(whence: __wasi_whence_t) -> &'static str {
    match whence {
        __WASI_WHENCE_CUR => "__WASI_WHENCE_CUR",
        __WASI_WHENCE_END => "__WASI_WHENCE_END",
        __WASI_WHENCE_SET => "__WASI_WHENCE_SET",
        other => panic!("Undefined whence value {:?}", other),
    }
}

pub const __WASI_DIRCOOKIE_START: __wasi_dircookie_t = 0;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bindgen_test_layout_wasi_dirent_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_dirent_t>(),
            24usize,
            concat!("Size of: ", stringify!(__wasi_dirent_t))
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
    fn bindgen_test_layout___wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_event_fd_readwrite_t>(),
            16usize,
            concat!("Size of: ", stringify!(__wasi_event_fd_readwrite_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_event_fd_readwrite_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_event_fd_readwrite_t))
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_event_fd_readwrite_t>())).nbytes as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_event_fd_readwrite_t),
                "::",
                stringify!(nbytes)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_event_fd_readwrite_t>())).flags as *const _ as usize
            },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_event_fd_readwrite_t),
                "::",
                stringify!(flags)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_event_t___wasi_event_u() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_event_u_t>(),
            16usize,
            concat!("Size of: ", stringify!(__wasi_event_u_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_event_u_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_event_u_t))
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_event_u_t>())).fd_readwrite as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_event_u_t),
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
            unsafe { &(*(::std::ptr::null::<__wasi_event_t>())).r#type as *const _ as usize },
            10usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_event_t),
                "::",
                stringify!(r#type)
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

    #[test]
    fn bindgen_test_layout_wasi_event_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_event_t>(),
            32usize,
            concat!("Size of: ", stringify!(__wasi_event_t))
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
            unsafe { &(*(::std::ptr::null::<__wasi_event_t>())).r#type as *const _ as usize },
            10usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_event_t),
                "::",
                stringify!(r#type)
            )
        );
    }

    #[test]
    fn bindgen_test_layout_wasi_fdstat_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_fdstat_t>(),
            24usize,
            concat!("Size of: ", stringify!(__wasi_fdstat_t))
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
    fn bindgen_test_layout_wasi_filestat_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_filestat_t>(),
            64usize,
            concat!("Size of: ", stringify!(__wasi_filestat_t))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).dev as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_dev)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).ino as *const _ as usize },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_ino)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).filetype as *const _ as usize },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_filetype)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).nlink as *const _ as usize },
            24usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_nlink)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).size as *const _ as usize },
            32usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_size)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).atim as *const _ as usize },
            40usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_atim)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).mtim as *const _ as usize },
            48usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_mtim)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).ctim as *const _ as usize },
            56usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_ctim)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_subscription_clock_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_subscription_clock_t>(),
            32usize,
            concat!("Size of: ", stringify!(__wasi_subscription_clock_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_subscription_clock_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_subscription_clock_t))
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_subscription_clock_t>())).id as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_clock_t),
                "::",
                stringify!(clock_id)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_subscription_clock_t>())).timeout as *const _ as usize
            },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_clock_t),
                "::",
                stringify!(timeout)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_subscription_clock_t>())).precision as *const _
                    as usize
            },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_clock_t),
                "::",
                stringify!(precision)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_subscription_clock_t>())).flags as *const _ as usize
            },
            24usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_clock_t),
                "::",
                stringify!(flags)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_fd_readwrite_t(
    ) {
        assert_eq!(
            ::std::mem::size_of::<__wasi_subscription_fd_readwrite_t>(),
            4usize,
            concat!("Size of: ", stringify!(__wasi_subscription_fd_readwrite_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_subscription_fd_readwrite_t>(),
            4usize,
            concat!(
                "Alignment of ",
                stringify!(__wasi_subscription_fd_readwrite_t)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_subscription_fd_readwrite_t>())).file_descriptor
                    as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_fd_readwrite_t),
                "::",
                stringify!(fd)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_subscription_t___wasi_subscription_u() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_subscription_u_t>(),
            32usize,
            concat!("Size of: ", stringify!(__wasi_subscription_u_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_subscription_u_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_subscription_u_t))
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_subscription_u_t>())).clock as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_u_t),
                "::",
                stringify!(clock)
            )
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_subscription_u_t>())).fd_readwrite as *const _
                    as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_u_t),
                "::",
                stringify!(fd_readwrite)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_subscription_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_subscription_t>(),
            48usize,
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
            unsafe {
                &(*(::std::ptr::null::<__wasi_subscription_t>())).r#type as *const _ as usize
            },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_subscription_t),
                "::",
                stringify!(r#type)
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
    fn bindgen_test_layout___wasi_filestat_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_filestat_t>(),
            64usize,
            concat!("Size of: ", stringify!(__wasi_filestat_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_filestat_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_filestat_t))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).dev as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_dev)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).ino as *const _ as usize },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_ino)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).filetype as *const _ as usize },
            16usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_filetype)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).nlink as *const _ as usize },
            24usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_nlink)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).size as *const _ as usize },
            32usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_size)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).atim as *const _ as usize },
            40usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_atim)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).mtim as *const _ as usize },
            48usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_filestat_t),
                "::",
                stringify!(st_mtim)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_filestat_t>())).ctim as *const _ as usize },
            56usize,
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
}
