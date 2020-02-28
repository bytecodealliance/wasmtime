use crate::{Error, Result};
use std::str;

/// Creates not-owned WASI path from byte slice.
///
/// NB WASI spec requires bytes to be valid UTF-8. Otherwise,
/// `__WASI_ERRNO_ILSEQ` error is returned.
pub(crate) fn path_from_slice<'a>(s: &'a [u8]) -> Result<&'a str> {
    str::from_utf8(s).map_err(|_| Error::EILSEQ)
}
