pub(crate) mod dir;
pub(crate) mod fadvise;
pub(crate) mod file;
pub(crate) mod filetime;
pub(crate) mod utimesat;

use crate::dir::SeekLoc;
use std::io::Result;

impl SeekLoc {
    #[cfg(target_pointer_width = "64")]
    pub unsafe fn from_raw(loc: i64) -> Result<Self> {
        let loc = loc.into();
        Ok(Self(loc))
    }
    #[cfg(target_pointer_width = "32")]
    pub unsafe fn from_raw(loc: i64) -> Result<Self> {
        // The cookie (or `loc`) is an opaque value, and applications aren't supposed to do
        // arithmetic on them or pick their own values or have any awareness of the numeric
        // range of the values. They're just supposed to pass back in the values that we
        // give them. And any value we give them will be convertable back to `long`,
        // because that's the type the OS gives them to us in. So return an `EINVAL`.
        use std::convert::TryInto;
        use std::io::Error;
        let loc = loc
            .try_into()
            .map_err(|_| Error::from_raw_os_error(libc::EINVAL))?;
        Ok(Self(loc))
    }
}
