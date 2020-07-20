use crate::dir::{Dir, Entry, EntryExt, SeekLoc};
use errno::{set_errno, Errno};
use std::{
    io::{Error, Result},
    ops::Deref,
};

#[derive(Copy, Clone, Debug)]
pub(crate) struct EntryImpl(libc::dirent);

impl Deref for EntryImpl {
    type Target = libc::dirent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl EntryExt for Entry {
    fn ino(&self) -> u64 {
        self.0.d_ino.into()
    }

    fn seek_loc(&self) -> Result<SeekLoc> {
        unsafe { SeekLoc::from_raw(self.0.d_off) }
    }
}

pub(crate) fn iter_impl(dir: &Dir) -> Option<Result<EntryImpl>> {
    set_errno(Errno(0));
    let dirent = unsafe { libc::readdir(dir.as_raw().as_ptr()) };
    if dirent.is_null() {
        let curr_errno = Error::last_os_error();
        if curr_errno.raw_os_error() != Some(0) {
            // A non-zero errno value was produced, so an error occurred.
            Some(Err(curr_errno))
        } else {
            // Not an error. We've simply reached the end of the stream.
            None
        }
    } else {
        Some(Ok(EntryImpl(unsafe { *dirent })))
    }
}
