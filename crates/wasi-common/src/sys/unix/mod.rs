pub(crate) mod clock;
pub(crate) mod fd;
pub(crate) mod osdir;
pub(crate) mod osfile;
pub(crate) mod oshandle;
pub(crate) mod osother;
pub(crate) mod path;
pub(crate) mod poll;
pub(crate) mod stdio;

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod linux;
        use linux as sys_impl;
    } else if #[cfg(target_os = "emscripten")] {
        mod emscripten;
        use emscripten as sys_impl;
    } else if #[cfg(any(target_os = "macos",
                        target_os = "netbsd",
                        target_os = "freebsd",
                        target_os = "openbsd",
                        target_os = "ios",
                        target_os = "dragonfly"))] {
        mod bsd;
        use bsd as sys_impl;
    }
}

use crate::handle::HandleRights;
use crate::sys::AsFile;
use crate::wasi::{types, RightsExt};
use crate::{Error, Result};
use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::os::unix::prelude::{AsRawFd, FileTypeExt, FromRawFd};
use std::path::Path;
use yanix::clock::ClockId;
use yanix::file::{AtFlags, OFlags};

pub(crate) use sys_impl::*;

impl<T: AsRawFd> AsFile for T {
    fn as_file(&self) -> io::Result<ManuallyDrop<File>> {
        let file = unsafe { File::from_raw_fd(self.as_raw_fd()) };
        Ok(ManuallyDrop::new(file))
    }
}

pub(super) fn get_file_type(file: &File) -> io::Result<types::Filetype> {
    let ft = file.metadata()?.file_type();
    let file_type = if ft.is_block_device() {
        log::debug!("Host fd {:?} is a block device", file.as_raw_fd());
        types::Filetype::BlockDevice
    } else if ft.is_char_device() {
        log::debug!("Host fd {:?} is a char device", file.as_raw_fd());
        types::Filetype::CharacterDevice
    } else if ft.is_dir() {
        log::debug!("Host fd {:?} is a directory", file.as_raw_fd());
        types::Filetype::Directory
    } else if ft.is_file() {
        log::debug!("Host fd {:?} is a file", file.as_raw_fd());
        types::Filetype::RegularFile
    } else if ft.is_socket() {
        log::debug!("Host fd {:?} is a socket", file.as_raw_fd());
        use yanix::socket::{get_socket_type, SockType};
        match unsafe { get_socket_type(file.as_raw_fd())? } {
            SockType::Datagram => types::Filetype::SocketDgram,
            SockType::Stream => types::Filetype::SocketStream,
            _ => return Err(io::Error::from_raw_os_error(libc::EINVAL)),
        }
    } else if ft.is_fifo() {
        log::debug!("Host fd {:?} is a fifo", file.as_raw_fd());
        types::Filetype::Unknown
    } else {
        log::debug!("Host fd {:?} is unknown", file.as_raw_fd());
        return Err(io::Error::from_raw_os_error(libc::EINVAL));
    };
    Ok(file_type)
}

pub(super) fn get_rights(file: &File, file_type: &types::Filetype) -> io::Result<HandleRights> {
    use yanix::{fcntl, file::OFlags};
    let (base, inheriting) = match file_type {
        types::Filetype::BlockDevice => (
            types::Rights::block_device_base(),
            types::Rights::block_device_inheriting(),
        ),
        types::Filetype::CharacterDevice => {
            use yanix::file::isatty;
            if unsafe { isatty(file.as_raw_fd())? } {
                (types::Rights::tty_base(), types::Rights::tty_base())
            } else {
                (
                    types::Rights::character_device_base(),
                    types::Rights::character_device_inheriting(),
                )
            }
        }
        types::Filetype::SocketDgram | types::Filetype::SocketStream => (
            types::Rights::socket_base(),
            types::Rights::socket_inheriting(),
        ),
        types::Filetype::SymbolicLink | types::Filetype::Unknown => (
            types::Rights::regular_file_base(),
            types::Rights::regular_file_inheriting(),
        ),
        types::Filetype::Directory => (
            types::Rights::directory_base(),
            types::Rights::directory_inheriting(),
        ),
        types::Filetype::RegularFile => (
            types::Rights::regular_file_base(),
            types::Rights::regular_file_inheriting(),
        ),
    };
    let mut rights = HandleRights::new(base, inheriting);
    let flags = unsafe { fcntl::get_status_flags(file.as_raw_fd())? };
    let accmode = flags & OFlags::ACCMODE;
    if accmode == OFlags::RDONLY {
        rights.base &= !types::Rights::FD_WRITE;
    } else if accmode == OFlags::WRONLY {
        rights.base &= !types::Rights::FD_READ;
    }
    Ok(rights)
}

pub fn preopen_dir<P: AsRef<Path>>(path: P) -> io::Result<File> {
    File::open(path)
}

impl From<types::Clockid> for ClockId {
    fn from(clock_id: types::Clockid) -> Self {
        use types::Clockid::*;
        match clock_id {
            Realtime => Self::Realtime,
            Monotonic => Self::Monotonic,
            ProcessCputimeId => Self::ProcessCPUTime,
            ThreadCputimeId => Self::ThreadCPUTime,
        }
    }
}

impl From<types::Fdflags> for OFlags {
    fn from(fdflags: types::Fdflags) -> Self {
        let mut nix_flags = Self::empty();
        if fdflags.contains(&types::Fdflags::APPEND) {
            nix_flags.insert(Self::APPEND);
        }
        if fdflags.contains(&types::Fdflags::DSYNC) {
            nix_flags.insert(Self::DSYNC);
        }
        if fdflags.contains(&types::Fdflags::NONBLOCK) {
            nix_flags.insert(Self::NONBLOCK);
        }
        if fdflags.contains(&types::Fdflags::RSYNC) {
            nix_flags.insert(O_RSYNC);
        }
        if fdflags.contains(&types::Fdflags::SYNC) {
            nix_flags.insert(Self::SYNC);
        }
        nix_flags
    }
}

impl From<OFlags> for types::Fdflags {
    fn from(oflags: OFlags) -> Self {
        let mut fdflags = Self::empty();
        if oflags.contains(OFlags::APPEND) {
            fdflags |= Self::APPEND;
        }
        if oflags.contains(OFlags::DSYNC) {
            fdflags |= Self::DSYNC;
        }
        if oflags.contains(OFlags::NONBLOCK) {
            fdflags |= Self::NONBLOCK;
        }
        if oflags.contains(O_RSYNC) {
            fdflags |= Self::RSYNC;
        }
        if oflags.contains(OFlags::SYNC) {
            fdflags |= Self::SYNC;
        }
        fdflags
    }
}

impl From<types::Oflags> for OFlags {
    fn from(oflags: types::Oflags) -> Self {
        let mut nix_flags = Self::empty();
        if oflags.contains(&types::Oflags::CREAT) {
            nix_flags.insert(Self::CREAT);
        }
        if oflags.contains(&types::Oflags::DIRECTORY) {
            nix_flags.insert(Self::DIRECTORY);
        }
        if oflags.contains(&types::Oflags::EXCL) {
            nix_flags.insert(Self::EXCL);
        }
        if oflags.contains(&types::Oflags::TRUNC) {
            nix_flags.insert(Self::TRUNC);
        }
        nix_flags
    }
}

impl TryFrom<libc::stat> for types::Filestat {
    type Error = Error;

    fn try_from(filestat: libc::stat) -> Result<Self> {
        fn filestat_to_timestamp(secs: u64, nsecs: u64) -> Result<types::Timestamp> {
            secs.checked_mul(1_000_000_000)
                .and_then(|sec_nsec| sec_nsec.checked_add(nsecs))
                .ok_or(Error::Overflow)
        }

        let filetype = yanix::file::FileType::from_stat_st_mode(filestat.st_mode);
        let dev = filestat.st_dev.try_into()?;
        let ino = filestat.st_ino.try_into()?;
        let atim = filestat_to_timestamp(
            filestat.st_atime.try_into()?,
            filestat.st_atime_nsec.try_into()?,
        )?;
        let ctim = filestat_to_timestamp(
            filestat.st_ctime.try_into()?,
            filestat.st_ctime_nsec.try_into()?,
        )?;
        let mtim = filestat_to_timestamp(
            filestat.st_mtime.try_into()?,
            filestat.st_mtime_nsec.try_into()?,
        )?;

        Ok(Self {
            dev,
            ino,
            nlink: filestat.st_nlink.into(),
            size: filestat.st_size as types::Filesize,
            atim,
            ctim,
            mtim,
            filetype: filetype.into(),
        })
    }
}

impl From<yanix::file::FileType> for types::Filetype {
    fn from(ft: yanix::file::FileType) -> Self {
        use yanix::file::FileType::*;
        match ft {
            RegularFile => Self::RegularFile,
            Symlink => Self::SymbolicLink,
            Directory => Self::Directory,
            BlockDevice => Self::BlockDevice,
            CharacterDevice => Self::CharacterDevice,
            /* Unknown | Socket | Fifo */
            _ => Self::Unknown,
            // TODO how to discriminate between STREAM and DGRAM?
            // Perhaps, we should create a more general WASI filetype
            // such as __WASI_FILETYPE_SOCKET, and then it would be
            // up to the client to check whether it's actually
            // STREAM or DGRAM?
        }
    }
}

impl From<types::Lookupflags> for AtFlags {
    fn from(flags: types::Lookupflags) -> Self {
        match flags {
            types::Lookupflags::SYMLINK_FOLLOW => Self::empty(),
            _ => Self::SYMLINK_NOFOLLOW,
        }
    }
}
