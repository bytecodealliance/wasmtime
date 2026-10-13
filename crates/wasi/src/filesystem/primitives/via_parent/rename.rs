use super::open_parent;
#[cfg(not(target_os = "linux"))]
use crate::filesystem::primitives::{FollowSymlinks, errors, stat_unchecked};
use crate::filesystem::primitives::{
    MaybeOwnedFile, path_has_trailing_slash, rename_unchecked, strip_dir_suffix,
};
use std::path::Path;
use std::{fs, io};

/// Implement `rename` by `open`ing up the parent component of the path and
/// then calling `rename_unchecked` on the last component.
pub(crate) fn rename(
    old_start: &fs::File,
    old_path: &Path,
    new_start: &fs::File,
    new_path: &Path,
) -> io::Result<()> {
    let old_start = MaybeOwnedFile::borrowed(old_start);
    let new_start = MaybeOwnedFile::borrowed(new_start);

    // As a special case, `rename` ignores a trailing slash rather than treating
    // it as equivalent to a trailing slash-dot, so strip any trailing slashes
    // for the purposes of `open_parent`.
    //
    // And remember whether either path ended with a slash so that we can still
    // fail if the source isn't a directory.
    let requires_dir = path_has_trailing_slash(old_path) || path_has_trailing_slash(new_path);
    let old_path = strip_dir_suffix(old_path);
    let new_path = strip_dir_suffix(new_path);

    let (old_dir, old_basename) = open_parent(old_start, &old_path)?;
    let (new_dir, new_basename) = open_parent(new_start, &new_path)?;

    // A trailing slash on either path requires the source to be a directory.
    //
    // On Linux, re-append a slash to the source and let the kernel check that
    // atomically with the rename. Linux never follows a symlink named with a
    // trailing slash in `renameat` and fails with `ENOTDIR` instead.
    #[cfg(target_os = "linux")]
    let mut concat;
    #[cfg(target_os = "linux")]
    let old_basename = if requires_dir {
        concat = old_basename.to_owned();
        concat.push("/");
        concat.as_os_str()
    } else {
        old_basename
    };

    // Elsewhere, such as on macOS and FreeBSD, `renameat` follows a symlink
    // named with a trailing slash. Check without following symlinks and rename
    // by the plain names instead, so such a symlink is never followed out of
    // `old_start`. The check isn't atomic with the rename, but whatever
    // replaces the source in between is still renamed by its plain name.
    #[cfg(not(target_os = "linux"))]
    if requires_dir
        && !stat_unchecked(&old_dir, Path::new(old_basename), FollowSymlinks::No)?.is_dir()
    {
        return Err(errors::is_not_directory());
    }

    rename_unchecked(
        &old_dir,
        old_basename.as_ref(),
        &new_dir,
        new_basename.as_ref(),
    )
}
