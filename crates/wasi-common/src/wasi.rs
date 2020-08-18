use crate::{Error, Result, WasiCtx};
use cfg_if::cfg_if;
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
            Error::IoError(e) => e.into(),
            Error::TooBig => Errno::TooBig,
            Error::Acces => Errno::Acces,
            Error::Badf => Errno::Badf,
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
            Error::Noent => Errno::Noent,
            Error::Nospc => Errno::Nospc,
            Error::Notdir => Errno::Notdir,
            Error::Notempty => Errno::Notempty,
            Error::Notsup => Errno::Notsup,
            Error::Overflow => Errno::Overflow,
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

// Turning an io::Error into an Errno is different on windows.
cfg_if! {
    if #[cfg(windows)] {
use winapi::shared::winerror;
use std::io;
impl From<io::Error> for Errno {
    fn from(err: io::Error) -> Self {
        match err.raw_os_error() {
            Some(code) => match code as u32 {
                winerror::ERROR_SUCCESS => Self::Success,
                winerror::ERROR_BAD_ENVIRONMENT => Self::TooBig,
                winerror::ERROR_FILE_NOT_FOUND => Self::Noent,
                winerror::ERROR_PATH_NOT_FOUND => Self::Noent,
                winerror::ERROR_TOO_MANY_OPEN_FILES => Self::Nfile,
                winerror::ERROR_ACCESS_DENIED => Self::Acces,
                winerror::ERROR_SHARING_VIOLATION => Self::Acces,
                winerror::ERROR_PRIVILEGE_NOT_HELD => Self::Notcapable,
                winerror::ERROR_INVALID_HANDLE => Self::Badf,
                winerror::ERROR_INVALID_NAME => Self::Noent,
                winerror::ERROR_NOT_ENOUGH_MEMORY => Self::Nomem,
                winerror::ERROR_OUTOFMEMORY => Self::Nomem,
                winerror::ERROR_DIR_NOT_EMPTY => Self::Notempty,
                winerror::ERROR_NOT_READY => Self::Busy,
                winerror::ERROR_BUSY => Self::Busy,
                winerror::ERROR_NOT_SUPPORTED => Self::Notsup,
                winerror::ERROR_FILE_EXISTS => Self::Exist,
                winerror::ERROR_BROKEN_PIPE => Self::Pipe,
                winerror::ERROR_BUFFER_OVERFLOW => Self::Nametoolong,
                winerror::ERROR_NOT_A_REPARSE_POINT => Self::Inval,
                winerror::ERROR_NEGATIVE_SEEK => Self::Inval,
                winerror::ERROR_DIRECTORY => Self::Notdir,
                winerror::ERROR_ALREADY_EXISTS => Self::Exist,
                x => {
                    log::debug!("winerror: unknown error value: {}", x);
                    Self::Io
                }
            },
            None => {
                log::debug!("Other I/O error: {}", err);
                Self::Io
            }
        }
    }
}

    } else {
use std::io;
impl From<io::Error> for Errno {
    fn from(err: io::Error) -> Self {
        match err.raw_os_error() {
            Some(code) => match code {
                libc::EPERM => Self::Perm,
                libc::ENOENT => Self::Noent,
                libc::ESRCH => Self::Srch,
                libc::EINTR => Self::Intr,
                libc::EIO => Self::Io,
                libc::ENXIO => Self::Nxio,
                libc::E2BIG => Self::TooBig,
                libc::ENOEXEC => Self::Noexec,
                libc::EBADF => Self::Badf,
                libc::ECHILD => Self::Child,
                libc::EAGAIN => Self::Again,
                libc::ENOMEM => Self::Nomem,
                libc::EACCES => Self::Acces,
                libc::EFAULT => Self::Fault,
                libc::EBUSY => Self::Busy,
                libc::EEXIST => Self::Exist,
                libc::EXDEV => Self::Xdev,
                libc::ENODEV => Self::Nodev,
                libc::ENOTDIR => Self::Notdir,
                libc::EISDIR => Self::Isdir,
                libc::EINVAL => Self::Inval,
                libc::ENFILE => Self::Nfile,
                libc::EMFILE => Self::Mfile,
                libc::ENOTTY => Self::Notty,
                libc::ETXTBSY => Self::Txtbsy,
                libc::EFBIG => Self::Fbig,
                libc::ENOSPC => Self::Nospc,
                libc::ESPIPE => Self::Spipe,
                libc::EROFS => Self::Rofs,
                libc::EMLINK => Self::Mlink,
                libc::EPIPE => Self::Pipe,
                libc::EDOM => Self::Dom,
                libc::ERANGE => Self::Range,
                libc::EDEADLK => Self::Deadlk,
                libc::ENAMETOOLONG => Self::Nametoolong,
                libc::ENOLCK => Self::Nolck,
                libc::ENOSYS => Self::Nosys,
                libc::ENOTEMPTY => Self::Notempty,
                libc::ELOOP => Self::Loop,
                libc::ENOMSG => Self::Nomsg,
                libc::EIDRM => Self::Idrm,
                libc::ENOLINK => Self::Nolink,
                libc::EPROTO => Self::Proto,
                libc::EMULTIHOP => Self::Multihop,
                libc::EBADMSG => Self::Badmsg,
                libc::EOVERFLOW => Self::Overflow,
                libc::EILSEQ => Self::Ilseq,
                libc::ENOTSOCK => Self::Notsock,
                libc::EDESTADDRREQ => Self::Destaddrreq,
                libc::EMSGSIZE => Self::Msgsize,
                libc::EPROTOTYPE => Self::Prototype,
                libc::ENOPROTOOPT => Self::Noprotoopt,
                libc::EPROTONOSUPPORT => Self::Protonosupport,
                libc::EAFNOSUPPORT => Self::Afnosupport,
                libc::EADDRINUSE => Self::Addrinuse,
                libc::EADDRNOTAVAIL => Self::Addrnotavail,
                libc::ENETDOWN => Self::Netdown,
                libc::ENETUNREACH => Self::Netunreach,
                libc::ENETRESET => Self::Netreset,
                libc::ECONNABORTED => Self::Connaborted,
                libc::ECONNRESET => Self::Connreset,
                libc::ENOBUFS => Self::Nobufs,
                libc::EISCONN => Self::Isconn,
                libc::ENOTCONN => Self::Notconn,
                libc::ETIMEDOUT => Self::Timedout,
                libc::ECONNREFUSED => Self::Connrefused,
                libc::EHOSTUNREACH => Self::Hostunreach,
                libc::EALREADY => Self::Already,
                libc::EINPROGRESS => Self::Inprogress,
                libc::ESTALE => Self::Stale,
                libc::EDQUOT => Self::Dquot,
                libc::ECANCELED => Self::Canceled,
                libc::EOWNERDEAD => Self::Ownerdead,
                libc::ENOTRECOVERABLE => Self::Notrecoverable,
                libc::ENOTSUP => Self::Notsup,
                x => {
                    log::debug!("Unknown errno value: {}", x);
                    Self::Io
                }
            },
            None => {
                log::debug!("Other I/O error: {}", err);
                Self::Io
            }
        }
    }
}
    }
}
