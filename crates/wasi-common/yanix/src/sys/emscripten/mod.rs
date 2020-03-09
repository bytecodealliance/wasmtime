#[path = "../linux/dir.rs"]
pub(crate) mod dir;
#[path = "../linux/fadvise.rs"]
pub(crate) mod fadvise;
#[path = "../linux/file.rs"]
pub(crate) mod file;
pub(crate) mod filetime;

use crate::dir::SeekLoc;
use std::convert::TryInto;
use std::io::{Error, Result};

impl SeekLoc {
    pub unsafe fn from_raw(loc: i64) -> Result<Self> {
        // The cookie (or `loc`) is an opaque value, and applications aren't supposed to do
        // arithmetic on them or pick their own values or have any awareness of the numeric
        // range of the values. They're just supposed to pass back in the values that we
        // give them. And any value we give them will be convertable back to `long`,
        // because that's the type the OS gives them to us in. So return an `EINVAL`.
        let loc = loc
            .try_into()
            .map_err(|_| Error::from_raw_os_error(libc::EINVAL))?;
        Ok(Self(loc))
    }
}
