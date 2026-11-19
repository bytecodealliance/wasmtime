use super::get_path::concatenate;
use std::path::Path;
use std::{fs, io};
use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS};

/// *Unsandboxed* function similar to `symlink`, but which does not perform
/// sandboxing.
pub(crate) fn symlink_unchecked(
    old_path: &Path,
    new_start: &fs::File,
    new_path: &Path,
    target_is_dir: impl FnOnce() -> bool,
) -> io::Result<()> {
    let new_full_path = concatenate(new_start, new_path)?;
    let result = if target_is_dir() {
        std::os::windows::fs::symlink_dir(old_path, &new_full_path)
    } else {
        std::os::windows::fs::symlink_file(old_path, &new_full_path)
    };

    match result {
        // Windows reports `ERROR_ACCESS_DENIED` when the link's path names an
        // existing directory, but on Unix that's reported as "already exists",
        // so frob the error code here to match Unix.
        Err(e)
            if e.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32)
                && fs::symlink_metadata(&new_full_path).is_ok() =>
        {
            Err(io::Error::from_raw_os_error(ERROR_ALREADY_EXISTS as i32))
        }
        other => other,
    }
}
