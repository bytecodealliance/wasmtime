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
//! For the convenience of consumers, `cap_std::fs::Dir` is re-exported from
//! this crate. This saves consumers tracking an additional dep on the exact
//! version of cap_std used by this crate, if they want to avoid it.
//!
//! The only place we expect to run into long-term compatibility issues
//! between `wasi-cap-std-sync` and the other impl crates that will come later
//! is in the `Sched` abstraction. Once we can build an async scheduler based
//! on Rust `Future`s, async impls will be able to interoperate, but the
//! synchronous scheduler depends on downcasting the `WasiFile` type down to
//! concrete types it knows about (which in turn impl `AsFd` for passing to
//! unix `poll`, or the analogous traits on windows).
//!
//! Why is this impl suffixed with `-sync`? Because `async` is coming soon!
//! The async impl may end up depending on tokio or other relatively heavy
//! deps, so we will retain a sync implementation so that wasi-common users
//! have an option of not pulling in an async runtime.

#![cfg_attr(io_lifetimes_use_std, feature(io_safety))]

pub mod clocks;
pub mod dir;
pub mod file;
pub mod net;
pub mod sched;
pub mod stdio;

pub use cap_std::ambient_authority;
pub use cap_std::fs::Dir;
pub use cap_std::net::TcpListener;
pub use clocks::clocks_ctx;
pub use sched::sched_ctx;

use crate::net::Socket;
use cap_rand::RngCore;
use std::path::Path;
use wasi_common::{file::FileCaps, table::Table, Error, WasiCtx, WasiFile};

pub struct WasiCtxBuilder(WasiCtx);

impl WasiCtxBuilder {
    pub fn new() -> Self {
        WasiCtxBuilder(WasiCtx::new(
            random_ctx(),
            clocks_ctx(),
            sched_ctx(),
            Table::new(),
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
    pub fn preopened_dir(mut self, dir: Dir, guest_path: impl AsRef<Path>) -> Result<Self, Error> {
        let dir = Box::new(crate::dir::Dir::from_cap_std(dir));
        self.0.push_preopened_dir(dir, guest_path)?;
        Ok(self)
    }
    pub fn preopened_socket(mut self, fd: u32, socket: impl Into<Socket>) -> Result<Self, Error> {
        let socket: Socket = socket.into();
        let file: Box<dyn WasiFile> = socket.into();

        let caps = FileCaps::FDSTAT_SET_FLAGS
            | FileCaps::FILESTAT_GET
            | FileCaps::READ
            | FileCaps::POLL_READWRITE;

        self.0.insert_file(fd, file, caps);
        Ok(self)
    }
    pub fn build(self) -> WasiCtx {
        self.0
    }
}

pub fn random_ctx() -> Box<dyn RngCore + Send + Sync> {
    Box::new(cap_rand::rngs::OsRng::default(ambient_authority()))
}
