use std::path::PathBuf;

pub trait WasiDir {}

pub(crate) struct DirEntry {
    pub(crate) flags: u32,
    pub(crate) preopen_path: Option<PathBuf>, // precondition: PathBuf is valid unicode
    pub(crate) dir: Box<dyn WasiDir>,
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
