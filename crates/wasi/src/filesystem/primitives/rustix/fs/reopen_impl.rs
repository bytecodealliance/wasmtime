use crate::filesystem::primitives::rustix::fs::file_path;
use crate::filesystem::primitives::{OpenOptions, open_unchecked};
use io_lifetimes::AsFilelike;
use rustix::fs::CWD;
use std::{fs, io};

/// Implementation of `reopen`.
pub(crate) fn reopen_impl(file: &fs::File, options: &OpenOptions) -> io::Result<fs::File> {
    if let Some(path) = file_path(file) {
        Ok(open_unchecked(
            &CWD.as_filelike_view::<fs::File>(),
            &path,
            options,
        )?)
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "Couldn't reopen file"))
    }
}
