//! The `wasi-cap-std-sync` crate provides impl of `WasiFile` and `WasiDir` in
//! terms of `cap_std::fs::{File, Dir}`. These types provide sandboxed access
//! to the local filesystem on both Unix and Windows.
//!
//! All syscalls are hidden behind the `cap-std` hierarchy, with the lone
//! exception of the `sched` implementation, which is provided for both unix
//! and windows in separate modules.
//!
//! Any `wasi_common::{WasiCtx, WasiCtxBuilder}` is interoperable with the
//! `wasi-cap-std-sync` crate. However, for convenience, `wasi-cap-std-sync`
//! provides its own `WasiCtxBuilder` that hooks up to all of the crate's
//! components, i.e. it fills in all of the arguments to
//! `WasiCtx::builder(...)`, presents `preopen_dir` in terms of
//! `cap_std::fs::Dir`, and provides convenience methods for inheriting the
//! parent process's stdio, args, and env.
//!
//! The only place we expect to run into long-term compatibility issues
//! between `wasi-cap-std-sync` and the other impl crates that will come later
//! is in the `Sched` abstraction. Once we can build an async scheduler based
//! on Rust `Future`s, async impls will be able to interoperate, but the
//! synchronous scheduler depends on downcasting the `WasiFile` type down to
//! concrete types it knows about (which in turn impl `AsRawFd` for passing to
//! unix `poll`, or the analogous traits on windows).
//!
//! Why is this impl suffixed with `-sync`? Because `async` is coming soon!
//! The async impl may end up depending on tokio or other relatively heavy
//! deps, so we will retain a sync implementation so that wasi-common users
//! have an option of not pulling in an async runtime.

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
