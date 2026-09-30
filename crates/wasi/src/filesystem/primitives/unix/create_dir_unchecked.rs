use rustix::fs::{Mode, mkdirat};
use std::path::Path;
use std::{fs, io};

/// *Unsandboxed* function similar to `create_dir`, but which does not perform
/// sandboxing.
pub(crate) fn create_dir_unchecked(start: &fs::File, path: &Path) -> io::Result<()> {
    // Mirror `std::fs::DirBuilder` by requesting a mode of `0o777`; the
    // process umask will then filter this down.
    Ok(mkdirat(start, path, Mode::RWXU | Mode::RWXG | Mode::RWXO)?)
}
