use crate::{dir::SeekLoc, Result};
use std::convert::TryInto;

impl SeekLoc {
    pub unsafe fn from_raw(loc: i64) -> Result<Self> {
        let loc = loc.try_into()?;
        Ok(Self(loc))
    }
}
