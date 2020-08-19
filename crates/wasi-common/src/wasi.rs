use crate::{Error, Result, WasiCtx};
use tracing::debug;

wiggle::from_witx!({
    witx: ["WASI/phases/snapshot/witx/wasi_snapshot_preview1.witx"],
    ctx: WasiCtx,
    errors: { errno => Error },
});

use types::Errno;

impl wiggle::GuestErrorType for Errno {
    fn success() -> Self {
        Self::Success
    }
}

impl types::GuestErrorConversion for WasiCtx {
    fn into_errno(&self, e: wiggle::GuestError) -> Errno {
        debug!("Guest error: {:?}", e);
        e.into()
    }
}

impl types::UserErrorConversion for WasiCtx {
    fn errno_from_error(&self, e: Error) -> Errno {
        debug!("Error: {:?}", e);
        e.into()
    }
}

impl From<Error> for Errno {
    fn from(e: Error) -> Errno {
        match e {
            Error::Guest(e) => e.into(),
            Error::TryFromInt(_) => Errno::Overflow,
            Error::Utf8(_) => Errno::Ilseq,
            Error::UnexpectedIo(_) => Errno::Io,
            Error::GetRandom(_) => Errno::Io,
            Error::TooBig => Errno::TooBig,
            Error::Acces => Errno::Acces,
            Error::Badf => Errno::Badf,
            Error::Busy => Errno::Busy,
            Error::Exist => Errno::Exist,
            Error::Fault => Errno::Fault,
            Error::Fbig => Errno::Fbig,
            Error::Ilseq => Errno::Ilseq,
            Error::Inval => Errno::Inval,
            Error::Io => Errno::Io,
            Error::Isdir => Errno::Isdir,
            Error::Loop => Errno::Loop,
            Error::Mfile => Errno::Mfile,
            Error::Mlink => Errno::Mlink,
            Error::Nametoolong => Errno::Nametoolong,
            Error::Nfile => Errno::Nfile,
            Error::Noent => Errno::Noent,
            Error::Nomem => Errno::Nomem,
            Error::Nospc => Errno::Nospc,
            Error::Notdir => Errno::Notdir,
            Error::Notempty => Errno::Notempty,
            Error::Notsup => Errno::Notsup,
            Error::Overflow => Errno::Overflow,
            Error::Pipe => Errno::Pipe,
            Error::Perm => Errno::Perm,
            Error::Spipe => Errno::Spipe,
            Error::Notcapable => Errno::Notcapable,
        }
    }
}

impl From<wiggle::GuestError> for Errno {
    fn from(err: wiggle::GuestError) -> Self {
        use wiggle::GuestError::*;
        match err {
            InvalidFlagValue { .. } => Self::Inval,
            InvalidEnumValue { .. } => Self::Inval,
            PtrOverflow { .. } => Self::Fault,
            PtrOutOfBounds { .. } => Self::Fault,
            PtrNotAligned { .. } => Self::Inval,
            PtrBorrowed { .. } => Self::Fault,
            InvalidUtf8 { .. } => Self::Ilseq,
            TryFromIntError { .. } => Self::Overflow,
            InFunc { .. } => Self::Inval,
            InDataField { .. } => Self::Inval,
            SliceLengthsDiffer { .. } => Self::Fault,
            BorrowCheckerOutOfHandles { .. } => Self::Fault,
        }
    }
}

impl From<std::fs::FileType> for types::Filetype {
    fn from(ftype: std::fs::FileType) -> Self {
        if ftype.is_file() {
            Self::RegularFile
        } else if ftype.is_dir() {
            Self::Directory
        } else if ftype.is_symlink() {
            Self::SymbolicLink
        } else {
            Self::Unknown
        }
    }
}

pub(crate) trait AsBytes {
    fn as_bytes(&self) -> Result<Vec<u8>>;
}

impl AsBytes for types::Dirent {
    fn as_bytes(&self) -> Result<Vec<u8>> {
        use std::convert::TryInto;
        use wiggle::GuestType;

        assert_eq!(
            Self::guest_size(),
            std::mem::size_of::<Self>() as _,
            "guest repr of types::Dirent and host repr should match"
        );

        let offset = Self::guest_size().try_into()?;
        let mut bytes: Vec<u8> = Vec::with_capacity(offset);
        bytes.resize(offset, 0);
        let ptr = bytes.as_mut_ptr() as *mut Self;
        unsafe { ptr.write_unaligned(self.clone()) };
        Ok(bytes)
    }
}

pub(crate) trait RightsExt: Sized {
    fn block_device_base() -> Self;
    fn block_device_inheriting() -> Self;
    fn character_device_base() -> Self;
    fn character_device_inheriting() -> Self;
    fn directory_base() -> Self;
    fn directory_inheriting() -> Self;
    fn regular_file_base() -> Self;
    fn regular_file_inheriting() -> Self;
    fn socket_base() -> Self;
    fn socket_inheriting() -> Self;
    fn tty_base() -> Self;
    fn tty_inheriting() -> Self;
}

impl RightsExt for types::Rights {
    // Block and character device interaction is outside the scope of
    // WASI. Simply allow everything.
    fn block_device_base() -> Self {
        Self::all()
    }
    fn block_device_inheriting() -> Self {
        Self::all()
    }
    fn character_device_base() -> Self {
        Self::all()
    }
    fn character_device_inheriting() -> Self {
        Self::all()
    }

    // Only allow directory operations on directories. Directories can only
    // yield file descriptors to other directories and files.
    fn directory_base() -> Self {
        Self::FD_FDSTAT_SET_FLAGS
            | Self::FD_SYNC
            | Self::FD_ADVISE
            | Self::PATH_CREATE_DIRECTORY
            | Self::PATH_CREATE_FILE
            | Self::PATH_LINK_SOURCE
            | Self::PATH_LINK_TARGET
            | Self::PATH_OPEN
            | Self::FD_READDIR
            | Self::PATH_READLINK
            | Self::PATH_RENAME_SOURCE
            | Self::PATH_RENAME_TARGET
            | Self::PATH_FILESTAT_GET
            | Self::PATH_FILESTAT_SET_SIZE
            | Self::PATH_FILESTAT_SET_TIMES
            | Self::FD_FILESTAT_GET
            | Self::FD_FILESTAT_SET_TIMES
            | Self::PATH_SYMLINK
            | Self::PATH_UNLINK_FILE
            | Self::PATH_REMOVE_DIRECTORY
            | Self::POLL_FD_READWRITE
    }
    fn directory_inheriting() -> Self {
        Self::all() ^ Self::SOCK_SHUTDOWN
    }

    // Operations that apply to regular files.
    fn regular_file_base() -> Self {
        Self::FD_DATASYNC
            | Self::FD_READ
            | Self::FD_SEEK
            | Self::FD_FDSTAT_SET_FLAGS
            | Self::FD_SYNC
            | Self::FD_TELL
            | Self::FD_WRITE
            | Self::FD_ADVISE
            | Self::FD_ALLOCATE
            | Self::FD_FILESTAT_GET
            | Self::FD_FILESTAT_SET_SIZE
            | Self::FD_FILESTAT_SET_TIMES
            | Self::POLL_FD_READWRITE
    }
    fn regular_file_inheriting() -> Self {
        Self::empty()
    }

    // Operations that apply to sockets and socket pairs.
    fn socket_base() -> Self {
        Self::FD_READ
            | Self::FD_FDSTAT_SET_FLAGS
            | Self::FD_WRITE
            | Self::FD_FILESTAT_GET
            | Self::POLL_FD_READWRITE
            | Self::SOCK_SHUTDOWN
    }
    fn socket_inheriting() -> Self {
        Self::all()
    }

    // Operations that apply to TTYs.
    fn tty_base() -> Self {
        Self::FD_READ
            | Self::FD_FDSTAT_SET_FLAGS
            | Self::FD_WRITE
            | Self::FD_FILESTAT_GET
            | Self::POLL_FD_READWRITE
    }
    fn tty_inheriting() -> Self {
        Self::empty()
    }
}
pub(crate) const DIRCOOKIE_START: types::Dircookie = 0;

impl crate::fdpool::Fd for types::Fd {
    fn as_raw(&self) -> u32 {
        (*self).into()
    }
    fn from_raw(raw_fd: u32) -> Self {
        Self::from(raw_fd)
    }
}
