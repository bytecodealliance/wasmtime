use crate::{Error, Result};
use std::convert::TryInto;
use std::ffi::CString;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn systemtime_to_timestamp(st: SystemTime) -> Result<u64> {
    st.duration_since(UNIX_EPOCH)
        .map_err(|_| Error::EINVAL)? // date earlier than UNIX_EPOCH
        .as_nanos()
        .try_into()
        .map_err(Into::into) // u128 doesn't fit into u64
}

pub(crate) fn str_to_cstring(s: &str) -> Result<CString> {
    CString::new(s.as_bytes()).map_err(|_| Error::EILSEQ)
}
