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
