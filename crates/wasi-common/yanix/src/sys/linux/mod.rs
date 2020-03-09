pub(crate) mod dir;
pub(crate) mod fadvise;
pub(crate) mod file;
pub(crate) mod filetime;
pub(crate) mod utimesat;

use crate::dir::SeekLoc;
use std::io::Result;

impl SeekLoc {
    pub unsafe fn from_raw(loc: i64) -> Result<Self> {
        let loc = loc.into();
        Ok(Self(loc))
    }
}
