use crate::{Error, WasiCtx};
use std::convert::{TryFrom, TryInto};
use tracing::debug;

wiggle::from_witx!({
    witx: ["$WASI_ROOT/phases/snapshot/witx/wasi_snapshot_preview1.witx"],
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
    fn errno_from_error(&self, e: Error) -> Result<Errno, wiggle::Trap> {
        debug!("Error: {:?}", e);
        e.try_into()
    }
}

impl TryFrom<Error> for Errno {
    type Error = wiggle::Trap;
    fn try_from(e: Error) -> Result<Errno, wiggle::Trap> {
        match e {
            Error::Guest(e) => Ok(e.into()),
            Error::TryFromInt(_) => Ok(Errno::Overflow),
            Error::Utf8(_) => Ok(Errno::Ilseq),
            Error::UnexpectedIo(_) => Ok(Errno::Io),
            Error::GetRandom(_) => Ok(Errno::Io),
            Error::TooBig => Ok(Errno::TooBig),
            Error::Acces => Ok(Errno::Acces),
            Error::Badf => Ok(Errno::Badf),
            Error::Busy => Ok(Errno::Busy),
            Error::Exist => Ok(Errno::Exist),
            Error::Fault => Ok(Errno::Fault),
            Error::Fbig => Ok(Errno::Fbig),
            Error::Ilseq => Ok(Errno::Ilseq),
            Error::Inval => Ok(Errno::Inval),
            Error::Io => Ok(Errno::Io),
            Error::Isdir => Ok(Errno::Isdir),
            Error::Loop => Ok(Errno::Loop),
            Error::Mfile => Ok(Errno::Mfile),
            Error::Mlink => Ok(Errno::Mlink),
            Error::Nametoolong => Ok(Errno::Nametoolong),
            Error::Nfile => Ok(Errno::Nfile),
            Error::Noent => Ok(Errno::Noent),
            Error::Nomem => Ok(Errno::Nomem),
            Error::Nospc => Ok(Errno::Nospc),
            Error::Notdir => Ok(Errno::Notdir),
            Error::Notempty => Ok(Errno::Notempty),
            Error::Notsup => Ok(Errno::Notsup),
            Error::Overflow => Ok(Errno::Overflow),
            Error::Pipe => Ok(Errno::Pipe),
            Error::Perm => Ok(Errno::Perm),
            Error::Spipe => Ok(Errno::Spipe),
            Error::Notcapable => Ok(Errno::Notcapable),
            Error::Unsupported(feature) => {
                Err(wiggle::Trap::String(format!("unsupported: {}", feature)))
            }
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
            InFunc { err, .. } => Errno::from(*err),
            InDataField { err, .. } => Errno::from(*err),
            SliceLengthsDiffer { .. } => Self::Fault,
            BorrowCheckerOutOfHandles { .. } => Self::Fault,
        }
    }
}

impl crate::fdpool::Fd for types::Fd {
    fn as_raw(&self) -> u32 {
        (*self).into()
    }
    fn from_raw(raw_fd: u32) -> Self {
        Self::from(raw_fd)
    }
}
