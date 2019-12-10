pub(crate) mod dir;
pub(crate) mod fadvise;
pub(crate) mod file;

use crate::{dir::SeekLoc, Result};

impl SeekLoc {
    pub unsafe fn from_raw(loc: i64) -> Result<Self> {
        let loc = loc.into();
        Ok(Self(loc))
    }
}
