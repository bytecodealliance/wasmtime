use crate::{host, Result};
use std::convert::TryInto;
use std::time::{SystemTime, UNIX_EPOCH};
pub(crate) fn systemtime_to_timestamp(st: SystemTime) -> Result<u64> {
    st.duration_since(UNIX_EPOCH)
        .map_err(|_| host::__WASI_EINVAL)? // date earlier than UNIX_EPOCH
        .as_nanos()
        .try_into()
        .map_err(|_| host::__WASI_EOVERFLOW) // u128 doesn't fit into u64
}
