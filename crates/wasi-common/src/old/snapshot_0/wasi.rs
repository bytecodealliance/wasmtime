//! Types and constants shared between 32-bit and 64-bit wasi. Types involving
//! pointer or `usize`-sized data are excluded here, so this file only contains
//! fixed-size types, so it's host/target independent.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use wig::witx_wasi_types;

witx_wasi_types!("old/snapshot_0" "wasi_unstable");

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

pub fn whence_to_str(whence: __wasi_whence_t) -> &'static str {
    match whence {
        __WASI_WHENCE_CUR => "__WASI_WHENCE_CUR",
        __WASI_WHENCE_END => "__WASI_WHENCE_END",
        __WASI_WHENCE_SET => "__WASI_WHENCE_SET",
        other => panic!("Undefined whence value {:?}", other),
    }
}

pub const __WASI_DIRCOOKIE_START: __wasi_dircookie_t = 0;
