#[path = "../linux/dir.rs"]
pub(crate) mod dir;
#[path = "../linux/fadvise.rs"]
pub(crate) mod fadvise;
#[path = "../linux/file.rs"]
pub(crate) mod file;

use crate::dir::SeekLoc;
use std::convert::TryInto;
use std::io::{Error, Result};

impl SeekLoc {
    pub unsafe fn from_raw(loc: i64) -> Result<Self> {
        let loc = loc
            .try_into()
            .map_err(|_| Error::from_raw_os_error(libc::EOVERFLOW))?;
        Ok(Self(loc))
    }
}
