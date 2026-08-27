use super::open_parent;
use crate::filesystem::primitives::{MaybeOwnedFile, SystemTimeSpec, set_times_nofollow_unchecked};
use std::path::Path;
use std::{fs, io};

#[inline]
pub(crate) fn set_times_nofollow(
    start: &fs::File,
    path: &Path,
    atime: Option<SystemTimeSpec>,
    mtime: Option<SystemTimeSpec>,
) -> io::Result<()> {
    let start = MaybeOwnedFile::borrowed(start);

    let (dir, basename) = open_parent(start, path)?;

    set_times_nofollow_unchecked(&dir, basename.as_ref(), atime, mtime)
}
