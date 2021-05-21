mod dir;
mod file;
pub mod sched;
pub mod stdio;

use std::cell::RefCell;
use std::future::Future;
use std::path::Path;
use std::rc::Rc;
pub use wasi_cap_std_sync::{clocks_ctx, random_ctx};
use wasi_common::{Error, Table, WasiCtx, WasiFile};

pub use dir::Dir;
pub use file::File;

use crate::sched::sched_ctx;

pub struct WasiCtxBuilder(WasiCtx);

impl WasiCtxBuilder {
    pub fn new() -> Self {
        WasiCtxBuilder(WasiCtx::new(
            random_ctx(),
            clocks_ctx(),
            sched_ctx(),
            Rc::new(RefCell::new(Table::new())),
        ))
    }
    pub fn env(mut self, var: &str, value: &str) -> Result<Self, wasi_common::StringArrayError> {
        self.0.push_env(var, value)?;
        Ok(self)
    }
    pub fn envs(mut self, env: &[(String, String)]) -> Result<Self, wasi_common::StringArrayError> {
        for (k, v) in env {
            self.0.push_env(k, v)?;
        }
        Ok(self)
    }
    pub fn inherit_env(mut self) -> Result<Self, wasi_common::StringArrayError> {
        for (key, value) in std::env::vars() {
            self.0.push_env(&key, &value)?;
        }
        Ok(self)
    }
    pub fn arg(mut self, arg: &str) -> Result<Self, wasi_common::StringArrayError> {
        self.0.push_arg(arg)?;
        Ok(self)
    }
    pub fn args(mut self, arg: &[String]) -> Result<Self, wasi_common::StringArrayError> {
        for a in arg {
            self.0.push_arg(&a)?;
        }
        Ok(self)
    }
    pub fn inherit_args(mut self) -> Result<Self, wasi_common::StringArrayError> {
        for arg in std::env::args() {
            self.0.push_arg(&arg)?;
        }
        Ok(self)
    }
    pub fn stdin(mut self, f: Box<dyn WasiFile>) -> Self {
        self.0.set_stdin(f);
        self
    }
    pub fn stdout(mut self, f: Box<dyn WasiFile>) -> Self {
        self.0.set_stdout(f);
        self
    }
    pub fn stderr(mut self, f: Box<dyn WasiFile>) -> Self {
        self.0.set_stderr(f);
        self
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
        mut self,
        dir: cap_std::fs::Dir,
        guest_path: impl AsRef<Path>,
    ) -> Result<Self, Error> {
        let dir = Box::new(crate::dir::Dir::from_cap_std(dir));
        self.0.push_preopened_dir(dir, guest_path)?;
        Ok(self)
    }
    pub fn build(self) -> WasiCtx {
        self.0
    }
}

// Much of this crate is implemented in terms of `async` methods from the
// wasi-cap-std-sync crate. These methods may be async in signature, however,
// they are synchronous in implementation (always Poll::Ready on first poll)
// and perform blocking syscalls.
//
// This function takes this blocking code and executes it using a dummy executor
// to assert its immediate readiness. We tell tokio this is a blocking operation
// with the block_in_place function.
pub(crate) fn block_on_dummy_executor<'a, F, Fut, T>(f: F) -> Result<T, Error>
where
    F: FnOnce() -> Fut + Send + 'a,
    Fut: Future<Output = Result<T, Error>>,
    T: Send + 'static,
{
    tokio::task::block_in_place(move || wiggle::run_in_dummy_executor(f()))
}
