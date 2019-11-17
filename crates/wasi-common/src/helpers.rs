use crate::{Error, Result};
use std::convert::TryInto;
use std::str;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn systemtime_to_timestamp(st: SystemTime) -> Result<u64> {
    st.duration_since(UNIX_EPOCH)
        .map_err(|_| Error::EINVAL)? // date earlier than UNIX_EPOCH
        .as_nanos()
        .try_into()
        .map_err(Into::into) // u128 doesn't fit into u64
}

/// Creates not-owned WASI path from byte slice.
///
/// NB WASI spec requires bytes to be valid UTF-8. Otherwise,
/// `__WASI_ERRNO_ILSEQ` error is returned.
pub(crate) fn path_from_slice<'a>(s: &'a [u8]) -> Result<&'a str> {
    str::from_utf8(s).map_err(|_| Error::EILSEQ)
}
