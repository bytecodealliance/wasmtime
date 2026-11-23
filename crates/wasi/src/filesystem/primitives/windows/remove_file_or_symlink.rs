use crate::filesystem::primitives::{
    Metadata, OpenOptions, OpenOptionsExt, open, remove_dir, remove_file,
};
use std::fs::File;
use std::io;
use std::path::Path;
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_ATTRIBUTE_DIRECTORY, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
};

/// Removes the file or symlink at `path`, relative to `start`.
///
/// On Windows a symlink to a directory must be removed with `remove_dir`
/// rather than `remove_file`, so this first opens the path to see which
/// kind of object it is.
pub(crate) fn remove_file_or_symlink(start: &File, path: &Path) -> io::Result<()> {
    // Note that `FILE_FLAG_OPEN_REPARSE_POINT` here means a trailing symlink
    // is opened as the reparse point itself rather than followed, so no
    // nofollow option is needed.
    let mut opts = OpenOptions::new();
    opts.access_mode(DELETE);
    opts.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS);
    let file = open(start, path, &opts)?;

    let meta = Metadata::from_file(&file)?;
    if meta.file_type().is_symlink()
        && meta.file_attributes() & FILE_ATTRIBUTE_DIRECTORY == FILE_ATTRIBUTE_DIRECTORY
    {
        remove_dir(start, path)?;
    } else {
        remove_file(start, path)?;
    }

    // Drop the file after calling `remove_file` or `remove_dir`, since
    // Windows doesn't actually remove the file until after the last open
    // handle is closed, and this protects us from race conditions where
    // other processes replace the file out from underneath us.
    drop(file);

    Ok(())
}
