use crate::clocks::WasiClocks;
use crate::dir::{DirCaps, DirEntry, WasiDir};
use crate::file::{FileCaps, FileEntry, WasiFile};
use crate::sched::WasiSched;
use crate::string_array::{StringArray, StringArrayError};
use crate::table::Table;
use crate::Error;
use cap_rand::RngCore;
use std::cell::{RefCell, RefMut};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct WasiCtx {
    pub args: StringArray,
    pub env: StringArray,
    pub random: RefCell<Box<dyn RngCore>>,
    pub clocks: WasiClocks,
    pub sched: Box<dyn WasiSched>,
    pub table: Rc<RefCell<Table>>,
}

impl WasiCtx {
    pub fn builder(
        random: RefCell<Box<dyn RngCore>>,
        clocks: WasiClocks,
        sched: Box<dyn WasiSched>,
        table: Rc<RefCell<Table>>,
    ) -> WasiCtxBuilder {
        WasiCtxBuilder(WasiCtx {
            args: StringArray::new(),
            env: StringArray::new(),
            random,
            clocks,
            sched,
            table,
        })
    }

    pub fn insert_file(&self, fd: u32, file: Box<dyn WasiFile>, caps: FileCaps) {
        self.table()
            .insert_at(fd, Box::new(FileEntry::new(caps, file)));
    }

    pub fn insert_dir(
        &self,
        fd: u32,
        dir: Box<dyn WasiDir>,
        caps: DirCaps,
        file_caps: FileCaps,
        path: PathBuf,
    ) {
        self.table().insert_at(
            fd,
            Box::new(DirEntry::new(caps, file_caps, Some(path), dir)),
        );
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

    pub fn arg(mut self, arg: &str) -> Result<Self, StringArrayError> {
        self.0.args.push(arg.to_owned())?;
        Ok(self)
    }

    pub fn env(mut self, var: &str, value: &str) -> Result<Self, StringArrayError> {
        self.0.env.push(format!("{}={}", var, value))?;
        Ok(self)
    }

    pub fn stdin(self, f: Box<dyn WasiFile>) -> Self {
        self.0.insert_file(
            0,
            f,
            FileCaps::READ | FileCaps::POLL_READWRITE, // XXX fixme: more rights are ok, but this is read-only
        );
        self
    }

    pub fn stdout(self, f: Box<dyn WasiFile>) -> Self {
        self.0.insert_file(
            1,
            f,
            FileCaps::WRITE | FileCaps::POLL_READWRITE, // XXX fixme: more rights are ok, but this is append only
        );
        self
    }

    pub fn stderr(self, f: Box<dyn WasiFile>) -> Self {
        self.0.insert_file(
            2,
            f,
            FileCaps::WRITE | FileCaps::POLL_READWRITE, // XXX fixme: more rights are ok, but this is append only
        );
        self
    }

    pub fn preopened_dir(
        self,
        dir: Box<dyn WasiDir>,
        path: impl AsRef<Path>,
    ) -> Result<Self, Error> {
        let caps = DirCaps::all();
        let file_caps = FileCaps::all();
        self.0.table().push(Box::new(DirEntry::new(
            caps,
            file_caps,
            Some(path.as_ref().to_owned()),
            dir,
        )))?;
        Ok(self)
    }
}
