use crate::file::{FileCaps, FileEntry, WasiFile};
use crate::table::Table;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

pub struct WasiCtx {
    table: Rc<RefCell<Table>>,
    preopen_paths: RefCell<HashMap<u32, Option<PathBuf>>>,
}

impl WasiCtx {
    pub fn new() -> Self {
        WasiCtx {
            table: Rc::new(RefCell::new(Table::new())),
            preopen_paths: RefCell::new(HashMap::new()),
        }
    }

    pub fn preopen_file(
        &self,
        fd: u32,
        file: Box<dyn WasiFile>,
        base_caps: FileCaps,
        inheriting_caps: FileCaps,
        path: Option<PathBuf>,
    ) {
        let e = FileEntry {
            base_caps,
            inheriting_caps,
            file,
        };
        self.table().insert_at(fd, e);
        self.preopen_paths.borrow_mut().insert(fd, path);
    }

    pub fn preopen_dir(&self, fd: u32, dir: Box<dyn WasiDir>, flags: u32, path: PathBuf) {
        let e = DirEntry { flags, dir };
        self.table().insert_at(fd, e);
        self.preopen_paths.borrow_mut().insert(fd, Some(path));
    }

    pub fn is_preopen(&self, fd: u32) -> bool {
        self.preopen_paths.borrow().contains_key(&fd)
    }

    pub fn table(&self) -> RefMut<Table> {
        self.table.borrow_mut()
    }
}

pub trait WasiDir {}

pub(crate) struct DirEntry {
    pub(crate) flags: u32,
    pub(crate) dir: Box<dyn WasiDir>,
}
