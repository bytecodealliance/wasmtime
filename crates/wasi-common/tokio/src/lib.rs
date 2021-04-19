mod dir;
mod file;
mod sched;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
pub use wasi_cap_std_sync::{clocks_ctx, random_ctx, Dir};
use wasi_common::{Error, Table, WasiCtx};

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
        dir: Dir,
        guest_path: impl AsRef<Path>,
    ) -> Result<Self, wasi_common::Error> {
        let dir = Box::new(crate::dir::Dir::from_cap_std(dir));
        Ok(WasiCtxBuilder(self.0.preopened_dir(dir, guest_path)?))
    }
    pub fn build(self) -> Result<WasiCtx, wasi_common::Error> {
        self.0.build()
    }
}

pub(crate) async fn asyncify<'a, F, T>(f: F) -> Result<T, Error>
where
    F: FnOnce() -> Result<T, std::io::Error> + Send + 'a,
    T: Send + 'static,
{
    // spawn_blocking requires a 'static function, but since we await on the
    // JoinHandle the lifetime of the spawn will be no longer than this function's body
    let f: Box<dyn FnOnce() -> Result<T, std::io::Error> + Send + 'a> = Box::new(f);
    let f = unsafe {
        std::mem::transmute::<_, Box<dyn FnOnce() -> Result<T, std::io::Error> + Send + 'static>>(f)
    };
    match tokio::task::spawn_blocking(|| f()).await {
        Ok(res) => Ok(res?),
        Err(_) => panic!("spawn_blocking died"),
    }
}
