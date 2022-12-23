use crate::clocks::WasiClocks;
use crate::dir::WasiDir;
use crate::file::WasiFile;
use crate::listener::WasiListener;
use crate::sched::WasiSched;
use crate::stream::WasiStream;
use crate::table::Table;
use crate::Error;
use cap_rand::RngCore;

pub struct WasiCtx {
    pub random: Box<dyn RngCore + Send + Sync>,
    pub clocks: WasiClocks,
    pub sched: Box<dyn WasiSched>,
    pub table: Table,
}

impl WasiCtx {
    pub fn new(
        random: Box<dyn RngCore + Send + Sync>,
        clocks: WasiClocks,
        sched: Box<dyn WasiSched>,
        table: Table,
    ) -> Self {
        let mut s = WasiCtx {
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

    pub fn insert_file(&mut self, fd: u32, file: Box<dyn WasiFile>) {
        self.table_mut().insert_at(fd, Box::new(file));
    }

    pub fn insert_stream(&mut self, fd: u32, stream: Box<dyn WasiStream>) {
        self.table_mut().insert_at(fd, Box::new(stream));
    }

    pub fn insert_listener(&mut self, fd: u32, listener: Box<dyn WasiListener>) {
        self.table_mut().insert_at(fd, Box::new(listener));
    }

    pub fn push_file(&mut self, file: Box<dyn WasiFile>) -> Result<u32, Error> {
        self.table_mut().push(Box::new(file))
    }

    pub fn insert_dir(&mut self, fd: u32, dir: Box<dyn WasiDir>) {
        self.table_mut().insert_at(fd, Box::new(dir))
    }

    pub fn push_dir(&mut self, dir: Box<dyn WasiDir>) -> Result<u32, Error> {
        self.table_mut().push(Box::new(dir))
    }

    pub fn table(&self) -> &Table {
        &self.table
    }

    pub fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }

    pub fn set_stdin(&mut self, s: Box<dyn WasiStream>) {
        self.insert_stream(0, s);
    }

    pub fn set_stdout(&mut self, s: Box<dyn WasiStream>) {
        self.insert_stream(1, s);
    }

    pub fn set_stderr(&mut self, s: Box<dyn WasiStream>) {
        self.insert_stream(2, s);
    }
}
