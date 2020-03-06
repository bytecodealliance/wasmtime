use crate::dir::{Dir, Entry, EntryExt, SeekLoc};
use std::{
    io::{Error, Result},
    ops::Deref,
};

#[derive(Copy, Clone, Debug)]
pub(crate) struct EntryImpl(libc::dirent64);

impl Deref for EntryImpl {
    type Target = libc::dirent64;

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
    let errno = Error::last_os_error();
    let dirent = unsafe { libc::readdir64(dir.as_raw().as_ptr()) };
    if dirent.is_null() {
        let curr_errno = Error::last_os_error();
        if errno.raw_os_error() != curr_errno.raw_os_error() {
            // TODO This should be verified on different BSD-flavours.
            //
            // According to 4.3BSD/POSIX.1-2001 man pages, there was an error
            // if the errno value has changed at some point during the sequence
            // of readdir calls.
            Some(Err(curr_errno))
        } else {
            // Not an error. We've simply reached the end of the stream.
            None
        }
    } else {
        Some(Ok(EntryImpl(unsafe { *dirent })))
    }
}
