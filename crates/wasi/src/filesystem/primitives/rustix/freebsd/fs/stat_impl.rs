use crate::filesystem::primitives::{FollowSymlinks, ImplMetadataExt, Metadata, manually};
use rustix::fs::{AtFlags, statat};
use std::path::Path;
use std::{fs, io};

pub(crate) fn stat_impl(
    start: &fs::File,
    path: &Path,
    follow: FollowSymlinks,
) -> io::Result<Metadata> {
    if !super::beneath_supported() {
        return manually::stat(start, path, follow);
    }

    let flags = AtFlags::RESOLVE_BENEATH
        | if follow == FollowSymlinks::Yes {
            AtFlags::empty()
        } else {
            AtFlags::SYMLINK_NOFOLLOW
        };
    Ok(ImplMetadataExt::from_rustix(statat(start, path, flags)?))
}
