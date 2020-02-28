//! WASI host types. These are types that contain raw pointers and `usize`
//! values, and so are platform-specific.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crate::old::snapshot_0::wasi::*;
use crate::old::snapshot_0::{Error, Result};
use std::{convert::TryInto, io, mem, slice};
use wig::witx_host_types;

witx_host_types!("old/snapshot_0" "wasi_unstable");

pub(crate) unsafe fn ciovec_to_host(ciovec: &__wasi_ciovec_t) -> io::IoSlice {
    let slice = slice::from_raw_parts(ciovec.buf as *const u8, ciovec.buf_len);
    io::IoSlice::new(slice)
}

pub(crate) unsafe fn iovec_to_host_mut(iovec: &mut __wasi_iovec_t) -> io::IoSliceMut {
    let slice = slice::from_raw_parts_mut(iovec.buf as *mut u8, iovec.buf_len);
    io::IoSliceMut::new(slice)
}

#[allow(dead_code)] // trouble with sockets
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub(crate) enum FileType {
    Unknown = __WASI_FILETYPE_UNKNOWN,
    BlockDevice = __WASI_FILETYPE_BLOCK_DEVICE,
    CharacterDevice = __WASI_FILETYPE_CHARACTER_DEVICE,
    Directory = __WASI_FILETYPE_DIRECTORY,
    RegularFile = __WASI_FILETYPE_REGULAR_FILE,
    SocketDgram = __WASI_FILETYPE_SOCKET_DGRAM,
    SocketStream = __WASI_FILETYPE_SOCKET_STREAM,
    Symlink = __WASI_FILETYPE_SYMBOLIC_LINK,
}

impl FileType {
    pub(crate) fn to_wasi(&self) -> __wasi_filetype_t {
        *self as __wasi_filetype_t
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Dirent {
    pub name: String,
    pub ftype: FileType,
    pub ino: u64,
    pub cookie: __wasi_dircookie_t,
}

impl Dirent {
    /// Serialize the directory entry to the format define by `__wasi_fd_readdir`,
    /// so that the serialized entries can be concatenated by the implementation.
    pub fn to_wasi_raw(&self) -> Result<Vec<u8>> {
        let name = self.name.as_bytes();
        let namlen = name.len();
        let dirent_size = mem::size_of::<__wasi_dirent_t>();
        let offset = dirent_size.checked_add(namlen).ok_or(Error::EOVERFLOW)?;

        let mut raw = Vec::<u8>::with_capacity(offset);
        raw.resize(offset, 0);

        let sys_dirent = raw.as_mut_ptr() as *mut __wasi_dirent_t;
        unsafe {
            sys_dirent.write_unaligned(__wasi_dirent_t {
                d_namlen: namlen.try_into()?,
                d_ino: self.ino,
                d_next: self.cookie,
                d_type: self.ftype.to_wasi(),
            });
        }

        let sys_name = unsafe { sys_dirent.offset(1) as *mut u8 };
        let sys_name = unsafe { slice::from_raw_parts_mut(sys_name, namlen) };
        sys_name.copy_from_slice(&name);

        Ok(raw)
    }
}
