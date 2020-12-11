// this file is extremely wip
#![allow(dead_code, unused_variables)]
use crate::error::Error;
use crate::file::{self, WasiFile};
use std::ops::Deref;
use std::path::PathBuf;

pub trait WasiDir {
    fn open_file(
        &self,
        symlink_follow: bool,
        path: &str,
        oflags: file::OFlags,
        fdflags: file::FdFlags,
    ) -> Result<Box<dyn WasiFile>, Error>;

    fn open_dir(
        &self,
        symlink_follow: bool,
        path: &str,
        create: bool,
    ) -> Result<Box<dyn WasiDir>, Error>;

    fn readdir(
        &self,
        cursor: ReaddirCursor,
    ) -> Result<Box<dyn Iterator<Item = Result<(ReaddirEntity, String), Error>>>, Error>;
}

pub(crate) struct DirEntry {
    pub(crate) base_caps: DirCaps,
    pub(crate) inheriting_caps: DirCaps,
    pub(crate) preopen_path: Option<PathBuf>, // precondition: PathBuf is valid unicode
    pub(crate) dir: Box<dyn WasiDir>,
}

impl DirEntry {
    pub fn get_cap(&self, caps: DirCaps) -> Result<&dyn WasiDir, Error> {
        if self.base_caps.contains(&caps) && self.inheriting_caps.contains(&caps) {
            Ok(self.dir.deref())
        } else {
            Err(Error::DirNotCapable(caps))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DirCaps {
    flags: u32,
}

impl DirCaps {
    pub fn empty() -> Self {
        DirCaps { flags: 0 }
    }

    /// Checks if `other` is a subset of those capabilties:
    pub fn contains(&self, other: &Self) -> bool {
        self.flags & other.flags == other.flags
    }

    pub const OPEN: Self = DirCaps { flags: 1 };
    pub const READDIR: Self = DirCaps { flags: 2 };
    pub const READLINK: Self = DirCaps { flags: 4 };
    pub const RENAME_SOURCE: Self = DirCaps { flags: 8 };
    pub const RENAME_TARGET: Self = DirCaps { flags: 16 };
    pub const SYMLINK: Self = DirCaps { flags: 32 };
    pub const REMOVE_DIRECTORY: Self = DirCaps { flags: 64 };
    pub const UNLINK_FILE: Self = DirCaps { flags: 128 };
}

impl std::fmt::Display for DirCaps {
    fn fmt(&self, _f: &mut std::fmt::Formatter) -> std::fmt::Result {
        todo!()
    }
}

pub trait TableDirExt {
    fn is_preopen(&self, fd: u32) -> bool;
}

impl TableDirExt for crate::table::Table {
    fn is_preopen(&self, fd: u32) -> bool {
        if self.is::<DirEntry>(fd) {
            let dir_entry: std::cell::RefMut<DirEntry> = self.get(fd).unwrap();
            dir_entry.preopen_path.is_some()
        } else {
            false
        }
    }
}

pub enum DirEntityType {
    File(crate::file::Filetype),
    Directory,
    SymbolicLink,
    Unknown,
}

pub struct ReaddirEntity {
    next: ReaddirCursor,
    inode: u64,
    namelen: u64,
    direnttype: DirEntityType,
}

pub struct ReaddirCursor(u64);
impl From<u64> for ReaddirCursor {
    fn from(c: u64) -> ReaddirCursor {
        ReaddirCursor(c)
    }
}
impl From<ReaddirCursor> for u64 {
    fn from(c: ReaddirCursor) -> u64 {
        c.0
    }
}

impl WasiDir for cap_std::fs::Dir {
    fn open_file(
        &self,
        symlink_follow: bool,
        path: &str,
        oflags: file::OFlags,
        fdflags: file::FdFlags,
    ) -> Result<Box<dyn WasiFile>, Error> {
        todo!()
    }

    fn open_dir(
        &self,
        symlink_follow: bool,
        path: &str,
        create: bool,
    ) -> Result<Box<dyn WasiDir>, Error> {
        todo!()
    }

    fn readdir(
        &self,
        cursor: ReaddirCursor,
    ) -> Result<Box<dyn Iterator<Item = Result<(ReaddirEntity, String), Error>>>, Error> {
        let rd = self
            .read_dir(PathBuf::new())?
            .enumerate()
            .skip(u64::from(cursor) as usize);
        Ok(Box::new(rd.map(|(ix, entry)| {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let direnttype = if file_type.is_dir() {
                DirEntityType::Directory
            } else if file_type.is_file() {
                DirEntityType::File(crate::file::Filetype::RegularFile) // XXX unify this with conversion in `impl WasiFile for cap_std::fs::File { get_filetype }`
            } else if file_type.is_symlink() {
                DirEntityType::SymbolicLink
            } else {
                DirEntityType::Unknown
            };
            let name = entry.file_name().into_string().map_err(|_| {
                Error::Utf8(todo!(
                    // XXX
                    "idk how to make utf8 error out of osstring conversion"
                ))
            })?;
            let namelen = name.as_bytes().len() as u64;
            // XXX need the metadata casing to be reusable here
            let inode = todo!();
            let entity = ReaddirEntity {
                next: ReaddirCursor::from(ix as u64 + 1),
                direnttype,
                inode,
                namelen,
            };
            Ok((entity, name))
        })))
    }
}
