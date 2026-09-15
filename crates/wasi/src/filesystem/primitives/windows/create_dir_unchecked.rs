use super::get_path::concatenate;
use std::path::Path;
use std::{fs, io};

/// *Unsandboxed* function similar to `create_dir`, but which does not perform
/// sandboxing.
///
pub(crate) fn create_dir_unchecked(start: &fs::File, path: &Path) -> io::Result<()> {
    let out_path = concatenate(start, path)?;
    fs::create_dir(out_path)
}
