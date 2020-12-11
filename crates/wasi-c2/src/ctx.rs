use crate::dir::{DirCaps, DirEntry, WasiDir};
use crate::file::{FileCaps, FileEntry, WasiFile};
use crate::table::Table;
use crate::Error;
use std::cell::{RefCell, RefMut};
use std::path::PathBuf;
use std::rc::Rc;

pub struct WasiCtx {
    table: Rc<RefCell<Table>>,
}

impl WasiCtx {
    pub fn builder() -> WasiCtxBuilder {
        WasiCtxBuilder(WasiCtx::new())
    }

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

    pub fn insert_dir(
        &self,
        fd: u32,
        dir: Box<dyn WasiDir>,
        base_caps: DirCaps,
        inheriting_caps: DirCaps,
        path: PathBuf,
    ) {
        let e = DirEntry {
            base_caps,
            inheriting_caps,
            preopen_path: Some(path),
            dir,
        };
        self.table().insert_at(fd, e);
    }

    pub fn table(&self) -> RefMut<Table> {
        self.table.borrow_mut()
    }
}

pub struct WasiCtxBuilder(WasiCtx);

impl WasiCtxBuilder {
    pub fn build(self) -> Result<WasiCtx, Error> {
        Ok(self.0)
    }
    pub fn arg(&mut self, _arg: &str) -> &mut Self {
        // Intentionally left blank. We do not handle arguments yet.
        self
    }
    pub fn inherit_stdio(&mut self) -> &mut Self {
        self.0.insert_file(
            0,
            Box::new(crate::stdio::stdin()),
            FileCaps::READ,
            FileCaps::READ,
        );
        self.0.insert_file(
            1,
            Box::new(crate::stdio::stdout()),
            FileCaps::WRITE,
            FileCaps::WRITE,
        );
        self.0.insert_file(
            2,
            Box::new(crate::stdio::stderr()),
            FileCaps::WRITE,
            FileCaps::WRITE,
        );
        self
    }
}
