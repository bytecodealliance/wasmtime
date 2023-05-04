use crate::{Table, TableError};

bitflags::bitflags! {
    pub struct FilePerms: usize {
        const READ = 0b1;
        const WRITE = 0b10;
    }
}

pub(crate) struct File {
    pub file: cap_std::fs::File,
    pub perms: FilePerms,
}

impl File {
    pub fn new(file: cap_std::fs::File) -> Self {
        Self {
            file,
            perms: FilePerms::READ | FilePerms::WRITE,
        }
    }
    pub fn read_only(file: cap_std::fs::File) -> Self {
        Self {
            file,
            perms: FilePerms::READ,
        }
    }
}
pub(crate) trait TableFileExt {
    fn push_file(&mut self, file: File) -> Result<u32, TableError>;
    fn delete_file(&mut self, fd: u32) -> Result<(), TableError>;

    fn is_file(&self, fd: u32) -> bool;
    fn get_file(&self, fd: u32) -> Result<&File, TableError>;
}

impl TableFileExt for Table {
    fn push_file(&mut self, file: File) -> Result<u32, TableError> {
        self.push(Box::new(file))
    }
    fn delete_file(&mut self, fd: u32) -> Result<(), TableError> {
        self.delete::<Box<File>>(fd)
    }

    fn is_file(&self, fd: u32) -> bool {
        self.is::<Box<File>>(fd)
    }
    fn get_file(&self, fd: u32) -> Result<&File, TableError> {
        self.get::<Box<File>>(fd).map(|d| d.as_ref())
    }
}
