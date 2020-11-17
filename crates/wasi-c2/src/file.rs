use crate::Error;
use std::ops::Deref;
use system_interface::fs::FileIoExt;

pub trait WasiFile: FileIoExt {}

pub(crate) struct FileEntry {
    pub(crate) base_caps: FileCaps,
    pub(crate) inheriting_caps: FileCaps,
    pub(crate) file: Box<dyn WasiFile>,
}

impl FileEntry {
    pub fn get_cap(&self, caps: FileCaps) -> Result<&dyn WasiFile, Error> {
        if self.base_caps.contains(&caps) && self.inheriting_caps.contains(&caps) {
            Ok(self.file.deref())
        } else {
            Err(Error::FileNotCapable(caps))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FileCaps {
    flags: u32,
}

impl FileCaps {
    pub fn empty() -> Self {
        FileCaps { flags: 0 }
    }

    /// Checks if `other` is a subset of those capabilties:
    pub fn contains(&self, other: &Self) -> bool {
        self.flags & other.flags == other.flags
    }

    pub const DATASYNC: Self = FileCaps { flags: 1 };
    pub const READ: Self = FileCaps { flags: 2 };
    pub const SEEK: Self = FileCaps { flags: 4 };
    pub const FDSTAT_SET_FLAGS: Self = FileCaps { flags: 8 };
    pub const SYNC: Self = FileCaps { flags: 16 };
    pub const TELL: Self = FileCaps { flags: 32 };
    pub const WRITE: Self = FileCaps { flags: 64 };
    pub const ADVISE: Self = FileCaps { flags: 128 };
    pub const ALLOCATE: Self = FileCaps { flags: 256 };
}

impl std::fmt::Display for FileCaps {
    fn fmt(&self, _f: &mut std::fmt::Formatter) -> std::fmt::Result {
        todo!()
    }
}
