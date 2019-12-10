use crate::{
    dir::{Dir, Entry, EntryExt, SeekLoc},
    Errno, Result,
};
use std::ops::Deref;

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

    fn seek_loc(&self) -> SeekLoc {
        unsafe { SeekLoc::from_raw(self.0.d_off) }
    }
}

pub(crate) unsafe fn iter_impl(dir: &Dir) -> Option<Result<EntryImpl>> {
    let errno = Errno::last();
    let dirent = libc::readdir64(dir.0.as_ptr());
    if dirent.is_null() {
        if errno != Errno::last() {
            // TODO This should be verified on different BSD-flavours.
            //
            // According to 4.3BSD/POSIX.1-2001 man pages, there was an error
            // if the errno value has changed at some point during the sequence
            // of readdir calls.
            Some(Err(Errno::last().into()))
        } else {
            // Not an error. We've simply reached the end of the stream.
            None
        }
    } else {
        Some(Ok(EntryImpl(*dirent)))
    }
}
