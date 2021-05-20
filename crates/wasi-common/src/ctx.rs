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
    pub fn new(
        random: RefCell<Box<dyn RngCore>>,
        clocks: WasiClocks,
        sched: Box<dyn WasiSched>,
        table: Rc<RefCell<Table>>,
    ) -> Self {
        let mut s = WasiCtx {
            args: StringArray::new(),
            env: StringArray::new(),
            random,
            clocks,
            sched,
            table,
        };
        s.set_stdin(Box::new(crate::pipe::ReadPipe::new(std::io::empty())));
        s.set_stdout(Box::new(crate::pipe::WritePipe::new(std::io::sink())));
        s.set_stderr(Box::new(crate::pipe::WritePipe::new(std::io::sink())));
        s
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

    pub fn push_arg(&mut self, arg: &str) -> Result<(), StringArrayError> {
        self.args.push(arg.to_owned())
    }

    pub fn push_env(&mut self, var: &str, value: &str) -> Result<(), StringArrayError> {
        self.env.push(format!("{}={}", var, value))?;
        Ok(())
    }

    pub fn set_stdin(&mut self, f: Box<dyn WasiFile>) {
        self.insert_file(0, f, FileCaps::all());
    }

    pub fn set_stdout(&mut self, f: Box<dyn WasiFile>) {
        self.insert_file(1, f, FileCaps::all());
    }

    pub fn set_stderr(&mut self, f: Box<dyn WasiFile>) {
        self.insert_file(2, f, FileCaps::all());
    }

    pub fn push_preopened_dir(
        &mut self,
        dir: Box<dyn WasiDir>,
        path: impl AsRef<Path>,
    ) -> Result<(), Error> {
        let caps = DirCaps::all();
        let file_caps = FileCaps::all();
        self.table().push(Box::new(DirEntry::new(
            caps,
            file_caps,
            Some(path.as_ref().to_owned()),
            dir,
        )))?;
        Ok(())
    }
}
