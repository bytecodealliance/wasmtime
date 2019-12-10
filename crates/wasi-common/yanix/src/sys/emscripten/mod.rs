#[path = "../linux/dir.rs"]
pub(crate) mod dir;
#[path = "../linux/fadvise.rs"]
pub(crate) mod fadvise;
#[path = "../linux/file.rs"]
pub(crate) mod file;

use crate::{dir::SeekLoc, Result};
use std::convert::TryInto;

impl SeekLoc {
    pub unsafe fn from_raw(loc: i64) -> Result<Self> {
        let loc = loc.try_into()?;
        Ok(Self(loc))
    }
}
