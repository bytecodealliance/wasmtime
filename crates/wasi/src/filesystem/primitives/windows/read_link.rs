use super::get_path::concatenate;
use std::path::{Path, PathBuf};
use std::{fs, io};

/// *Unsandboxed* function similar to `read_link`, but which does not perform
/// sandboxing.
pub(crate) fn read_link(start: &fs::File, path: &Path) -> io::Result<PathBuf> {
    let out_path = concatenate(start, path)?;
    fs::read_link(out_path)
}
