use crate::dir::{DirCaps, DirEntry, WasiDir};
use crate::file::{FileCaps, FileEntry, WasiFile};
use crate::string_array::{StringArray, StringArrayError};
use crate::table::Table;
use crate::Error;
use std::cell::{RefCell, RefMut};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct WasiCtx {
    pub(crate) args: StringArray,
    pub(crate) env: StringArray,
    table: Rc<RefCell<Table>>,
}

impl WasiCtx {
    pub fn builder() -> WasiCtxBuilder {
        WasiCtxBuilder(WasiCtx::new())
    }

    pub fn new() -> Self {
        WasiCtx {
            args: StringArray::new(),
            env: StringArray::new(),
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

    pub fn arg(&mut self, arg: &str) -> Result<&mut Self, StringArrayError> {
        self.0.args.push(arg.to_owned())?;
        Ok(self)
    }

    pub fn stdin(&mut self, f: Box<dyn WasiFile>) -> &mut Self {
        self.0.insert_file(
            0,
            f,
            FileCaps::READ, // XXX probably more rights are ok
            FileCaps::READ,
        );
        self
    }

    pub fn stdout(&mut self, f: Box<dyn WasiFile>) -> &mut Self {
        self.0.insert_file(
            1,
            f,
            FileCaps::WRITE, // XXX probably more rights are ok
            FileCaps::WRITE,
        );
        self
    }

    pub fn stderr(&mut self, f: Box<dyn WasiFile>) -> &mut Self {
        self.0.insert_file(
            2,
            f,
            FileCaps::WRITE, // XXX probably more rights are ok
            FileCaps::WRITE,
        );
        self
    }

    pub fn inherit_stdio(&mut self) -> &mut Self {
        self.stdin(Box::new(crate::stdio::stdin()))
            .stdout(Box::new(crate::stdio::stdout()))
            .stderr(Box::new(crate::stdio::stderr()))
    }

    pub fn preopened_dir(
        &mut self,
        dir: Box<dyn WasiDir>,
        path: impl AsRef<Path>,
    ) -> Result<&mut Self, Error> {
        let base_caps = DirCaps::OPEN;
        let inheriting_caps = DirCaps::OPEN;
        self.0.table().push(DirEntry {
            base_caps,
            inheriting_caps,
            preopen_path: Some(path.as_ref().to_owned()),
            dir,
        })?;
        Ok(self)
    }
}
