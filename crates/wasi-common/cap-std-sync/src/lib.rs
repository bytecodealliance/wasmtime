pub mod clocks;
pub mod dir;
pub mod file;
pub mod sched;
pub mod stdio;

pub use clocks::clocks_ctx;
pub use sched::sched_ctx;

use cap_rand::RngCore;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use wasi_common::{table::Table, Error, WasiCtx, WasiFile};

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
    pub fn stdin(self, f: Box<dyn WasiFile>) -> Self {
        WasiCtxBuilder(self.0.stdin(f))
    }
    pub fn stdout(self, f: Box<dyn WasiFile>) -> Self {
        WasiCtxBuilder(self.0.stdout(f))
    }
    pub fn stderr(self, f: Box<dyn WasiFile>) -> Self {
        WasiCtxBuilder(self.0.stderr(f))
    }
    pub fn inherit_stdin(self) -> Self {
        self.stdin(Box::new(crate::stdio::stdin()))
    }
    pub fn inherit_stdout(self) -> Self {
        self.stdout(Box::new(crate::stdio::stdout()))
    }
    pub fn inherit_stderr(self) -> Self {
        self.stderr(Box::new(crate::stdio::stderr()))
    }
    pub fn inherit_stdio(self) -> Self {
        self.inherit_stdin().inherit_stdout().inherit_stderr()
    }
    pub fn preopened_dir(
        self,
        dir: cap_std::fs::Dir,
        path: impl AsRef<Path>,
    ) -> Result<Self, Error> {
        let dir = Box::new(crate::dir::Dir::from_cap_std(dir));
        Ok(WasiCtxBuilder(self.0.preopened_dir(dir, path)?))
    }
    pub fn build(self) -> Result<WasiCtx, Error> {
        self.0.build()
    }
}

pub fn random_ctx() -> RefCell<Box<dyn RngCore>> {
    RefCell::new(Box::new(unsafe { cap_rand::rngs::OsRng::default() }))
}
