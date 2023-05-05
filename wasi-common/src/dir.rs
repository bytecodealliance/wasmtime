use crate::{FilePerms, Table, TableError};

bitflags::bitflags! {
    pub struct DirPerms: usize {
        const READ = 0b1;
        const MUTATE = 0b10;
    }
}

pub(crate) struct Dir {
    pub dir: cap_std::fs::Dir,
    pub perms: DirPerms,
    pub file_perms: FilePerms,
}

impl Dir {
    pub fn new(dir: cap_std::fs::Dir, perms: DirPerms, file_perms: FilePerms) -> Self {
        Dir {
            dir,
            perms,
            file_perms,
        }
    }
}

pub(crate) trait TableDirExt {
    fn push_dir(&mut self, dir: Dir) -> Result<u32, TableError>;
    fn delete_dir(&mut self, fd: u32) -> Result<(), TableError>;
    fn is_dir(&self, fd: u32) -> bool;
    fn get_dir(&self, fd: u32) -> Result<&Dir, TableError>;
}

impl TableDirExt for Table {
    fn push_dir(&mut self, dir: Dir) -> Result<u32, TableError> {
        self.push(Box::new(dir))
    }
    fn delete_dir(&mut self, fd: u32) -> Result<(), TableError> {
        self.delete::<Box<Dir>>(fd)
    }
    fn is_dir(&self, fd: u32) -> bool {
        self.is::<Box<Dir>>(fd)
    }
    fn get_dir(&self, fd: u32) -> Result<&Dir, TableError> {
        self.get::<Box<Dir>>(fd).map(|d| d.as_ref())
    }
}
