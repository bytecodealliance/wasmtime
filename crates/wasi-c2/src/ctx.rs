use crate::clocks::{WasiMonotonicClock, WasiSystemClock};
use crate::dir::{DirCaps, DirEntry, WasiDir};
use crate::file::{FileCaps, FileEntry, WasiFile};
use crate::sched::{SyncSched, WasiSched};
use crate::string_array::{StringArray, StringArrayError};
use crate::table::Table;
use crate::Error;
use cap_rand::RngCore;
use std::cell::{RefCell, RefMut};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct WasiCtx {
    pub(crate) args: StringArray,
    pub(crate) env: StringArray,
    pub(crate) random: RefCell<Box<dyn RngCore>>,
    pub(crate) clocks: WasiCtxClocks,
    pub(crate) sched: Box<dyn WasiSched>,
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
            random: RefCell::new(Box::new(unsafe { cap_rand::rngs::OsRng::default() })),
            clocks: WasiCtxClocks::default(),
            sched: Box::new(SyncSched::default()),
            table: Rc::new(RefCell::new(Table::new())),
        }
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

    pub fn stdin(mut self, f: Box<dyn WasiFile>) -> Self {
        self.0.insert_file(
            0,
            f,
            FileCaps::READ, // XXX fixme: more rights are ok, but this is read-only
        );
        self
    }

    pub fn stdout(mut self, f: Box<dyn WasiFile>) -> Self {
        self.0.insert_file(
            1,
            f,
            FileCaps::WRITE, // XXX fixme: more rights are ok, but this is append only
        );
        self
    }

    pub fn stderr(mut self, f: Box<dyn WasiFile>) -> Self {
        self.0.insert_file(
            2,
            f,
            FileCaps::WRITE, // XXX fixme: more rights are ok, but this is append only
        );
        self
    }

    pub fn inherit_stdio(self) -> Self {
        self.stdin(Box::new(crate::stdio::stdin()))
            .stdout(Box::new(crate::stdio::stdout()))
            .stderr(Box::new(crate::stdio::stderr()))
    }

    pub fn preopened_dir(
        mut self,
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

    pub fn random(mut self, random: Box<dyn RngCore>) -> Self {
        self.0.random.replace(random);
        self
    }
}

pub struct WasiCtxClocks {
    pub(crate) system: Box<dyn WasiSystemClock>,
    pub(crate) monotonic: Box<dyn WasiMonotonicClock>,
    pub(crate) creation_time: cap_std::time::Instant,
}

impl Default for WasiCtxClocks {
    fn default() -> WasiCtxClocks {
        let system = Box::new(unsafe { cap_std::time::SystemClock::new() });
        let monotonic = unsafe { cap_std::time::MonotonicClock::new() };
        let creation_time = monotonic.now();
        let monotonic = Box::new(monotonic);
        WasiCtxClocks {
            system,
            monotonic,
            creation_time,
        }
    }
}
