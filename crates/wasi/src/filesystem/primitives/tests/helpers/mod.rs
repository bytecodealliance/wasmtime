//! Shared helpers for the tests imported from `cap-std`.
//!
//! These tests were written against `cap_std::fs::Dir`, whose methods bundle up
//! several `crate::filesystem::primitives` calls. Anything that is a bare
//! forward to a primitive is called as `p::foo(..)` directly at the call site;
//! only the operations that add options, flags, or logic live here.

use crate::filesystem::primitives as p;
use ambient_authority::ambient_authority;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Open a temporary directory as a start-directory handle.
pub fn dir_of(t: &TempDir) -> File {
    p::open_ambient_dir(t.path(), ambient_authority()).unwrap()
}

/// `open_ambient_dir` with the ambient authority supplied.
pub fn open_ambient_dir(path: impl AsRef<Path>) -> io::Result<File> {
    p::open_ambient_dir(path.as_ref(), ambient_authority())
}

/// `Dir::create`: open for writing, creating and truncating.
pub fn create(d: &File, path: impl AsRef<Path>) -> io::Result<File> {
    p::open(
        d,
        path.as_ref(),
        p::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true),
    )
}

/// `Dir::open`: open for reading.
pub fn open(d: &File, path: impl AsRef<Path>) -> io::Result<File> {
    p::open(d, path.as_ref(), p::OpenOptions::new().read(true))
}

/// `Dir::create_dir`, with the default `DirOptions`.
pub fn create_dir(d: &File, path: impl AsRef<Path>) -> io::Result<()> {
    p::create_dir(d, path.as_ref(), &p::DirOptions::new())
}

/// `Dir::create_dir_all`, in terms of the single-level `create_dir`.
pub fn create_dir_all(d: &File, path: impl AsRef<Path>) -> io::Result<()> {
    let mut acc = PathBuf::new();
    for component in path.as_ref().components() {
        acc.push(component);
        match p::create_dir(d, &acc, &p::DirOptions::new()) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// `Dir::metadata`: stat, following symlinks.
pub fn metadata(d: &File, path: impl AsRef<Path>) -> io::Result<p::Metadata> {
    p::stat(d, path.as_ref(), p::FollowSymlinks::Yes)
}

/// `Dir::symlink_metadata`: stat, without following symlinks.
pub fn symlink_metadata(d: &File, path: impl AsRef<Path>) -> io::Result<p::Metadata> {
    p::stat(d, path.as_ref(), p::FollowSymlinks::No)
}

/// `DirExt::open_dir_nofollow`.
pub fn open_dir_nofollow(d: &File, path: impl AsRef<Path>) -> io::Result<File> {
    p::open(
        d,
        path.as_ref(),
        p::dir_options().follow(p::FollowSymlinks::No),
    )
}

/// `Dir::read_dir`: open the subdirectory, then read its entries.
pub fn read_dir(d: &File, path: impl AsRef<Path>) -> io::Result<super::super::read_dir::ReadDir> {
    p::read_base_dir(&p::open_dir(d, path.as_ref())?)
}

pub fn exists(d: &File, path: impl AsRef<Path>) -> bool {
    metadata(d, path).is_ok()
}

pub fn is_dir(d: &File, path: impl AsRef<Path>) -> bool {
    metadata(d, path).map(|m| m.is_dir()).unwrap_or(false)
}

pub fn is_file(d: &File, path: impl AsRef<Path>) -> bool {
    metadata(d, path)
        .map(|m| m.file_type().is_file())
        .unwrap_or(false)
}

pub fn write(d: &File, path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    use std::io::Write;
    let mut f = create(d, path)?;
    f.write_all(contents.as_ref())
}

pub fn read(d: &File, path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    use std::io::Read;
    let mut f = open(d, path)?;
    let mut v = Vec::new();
    f.read_to_end(&mut v)?;
    Ok(v)
}

pub fn read_to_string(d: &File, path: impl AsRef<Path>) -> io::Result<String> {
    use std::io::Read;
    let mut f = open(d, path)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}

/// `DirExt::symlink`. On Unix there is only one flavour of symlink; on Windows
/// the file/dir distinction matters, so these dispatch accordingly.
#[cfg(not(windows))]
pub fn symlink(d: &File, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    p::symlink(src.as_ref(), d, dst.as_ref())
}

#[cfg(windows)]
pub fn symlink(d: &File, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    p::symlink_file(src.as_ref(), d, dst.as_ref())
}

#[cfg(not(windows))]
pub fn symlink_file(d: &File, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    p::symlink(src.as_ref(), d, dst.as_ref())
}

#[cfg(windows)]
pub fn symlink_file(d: &File, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    p::symlink_file(src.as_ref(), d, dst.as_ref())
}

#[cfg(not(windows))]
pub fn symlink_dir(d: &File, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    p::symlink(src.as_ref(), d, dst.as_ref())
}

#[cfg(windows)]
pub fn symlink_dir(d: &File, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    p::symlink_dir(src.as_ref(), d, dst.as_ref())
}
