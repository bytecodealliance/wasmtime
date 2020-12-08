use crate::dir::{DirEntry, WasiDir};
use crate::file::{FileCaps, FileEntry, WasiFile};
use crate::table::Table;
use std::cell::{RefCell, RefMut};
use std::path::PathBuf;
use std::rc::Rc;

pub struct WasiCtx {
    table: Rc<RefCell<Table>>,
}

impl WasiCtx {
    pub fn new() -> Self {
        WasiCtx {
            table: Rc::new(RefCell::new(Table::new())),
        }
    }

    pub fn insert_file(
        &self,
        fd: u32,
        file: Box<dyn WasiFile>,
        base_caps: FileCaps,
        inheriting_caps: FileCaps,
    ) {
        let e = FileEntry {
            base_caps,
            inheriting_caps,
            file,
        };
        self.table().insert_at(fd, e);
    }

    pub fn insert_dir(&self, fd: u32, dir: Box<dyn WasiDir>, flags: u32, path: PathBuf) {
        let e = DirEntry {
            flags,
            preopen_path: Some(path),
            dir,
        };
        self.table().insert_at(fd, e);
    }

    pub fn table(&self) -> RefMut<Table> {
        self.table.borrow_mut()
    }
}
