mod dir;
mod file;
pub mod sched;
pub mod stdio;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
pub use wasi_cap_std_sync::{clocks_ctx, random_ctx};
use wasi_common::{Error, Table, WasiCtx};

pub use dir::Dir;
pub use file::File;

use crate::sched::sched_ctx;

pub struct WasiCtxBuilder(wasi_common::WasiCtxBuilder);

impl WasiCtxBuilder {
    pub fn new() -> Self {
        WasiCtxBuilder(WasiCtx::builder(
            random_ctx(),
            clocks_ctx(),
            sched_ctx(),
            Rc::new(RefCell::new(Table::new())),
        ))
    }
    pub fn env(self, var: &str, value: &str) -> Result<Self, wasi_common::StringArrayError> {
        let s = self.0.env(var, value)?;
        Ok(WasiCtxBuilder(s))
    }
    pub fn envs(self, env: &[(String, String)]) -> Result<Self, wasi_common::StringArrayError> {
        let mut s = self;
        for (k, v) in env {
            s = s.env(k, v)?;
        }
        Ok(s)
    }
    pub fn inherit_env(self) -> Result<Self, wasi_common::StringArrayError> {
        let mut s = self.0;
        for (key, value) in std::env::vars() {
            s = s.env(&key, &value)?;
        }
        Ok(WasiCtxBuilder(s))
    }
    pub fn arg(self, arg: &str) -> Result<Self, wasi_common::StringArrayError> {
        let s = self.0.arg(arg)?;
        Ok(WasiCtxBuilder(s))
    }
    pub fn args(self, arg: &[String]) -> Result<Self, wasi_common::StringArrayError> {
        let mut s = self;
        for a in arg {
            s = s.arg(&a)?;
        }
        Ok(s)
    }
    pub fn inherit_args(self) -> Result<Self, wasi_common::StringArrayError> {
        let mut s = self.0;
        for arg in std::env::args() {
            s = s.arg(&arg)?;
        }
        Ok(WasiCtxBuilder(s))
    }
    pub fn stdin(self, f: Box<dyn wasi_common::WasiFile>) -> Self {
        WasiCtxBuilder(self.0.stdin(f))
    }
    pub fn stdout(self, f: Box<dyn wasi_common::WasiFile>) -> Self {
        WasiCtxBuilder(self.0.stdout(f))
    }
    pub fn stderr(self, f: Box<dyn wasi_common::WasiFile>) -> Self {
        WasiCtxBuilder(self.0.stderr(f))
    }
    // XXX our crate needs its own stdios
    pub fn inherit_stdin(self) -> Self {
        self.stdin(Box::new(wasi_cap_std_sync::stdio::stdin()))
    }
    pub fn inherit_stdout(self) -> Self {
        self.stdout(Box::new(wasi_cap_std_sync::stdio::stdout()))
    }
    pub fn inherit_stderr(self) -> Self {
        self.stderr(Box::new(wasi_cap_std_sync::stdio::stderr()))
    }
    pub fn inherit_stdio(self) -> Self {
        self.inherit_stdin().inherit_stdout().inherit_stderr()
    }
    pub fn preopened_dir(
        self,
        dir: cap_std::fs::Dir,
        guest_path: impl AsRef<Path>,
    ) -> Result<Self, wasi_common::Error> {
        let dir = Box::new(Dir::from_cap_std(dir));
        Ok(WasiCtxBuilder(self.0.preopened_dir(dir, guest_path)?))
    }
    pub fn build(self) -> Result<WasiCtx, wasi_common::Error> {
        self.0.build()
    }
}

pub(crate) fn asyncify<'a, F, T>(f: F) -> Result<T, Error>
where
    F: FnOnce() -> Result<T, std::io::Error> + Send + 'a,
    T: Send + 'static,
{
    tokio::task::block_in_place(f).map_err(Into::into)
}
