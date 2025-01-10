//! The `wasi-cap-std-sync` crate provides impl of `WasiFile` and `WasiDir` in
//! terms of `cap_std::fs::{File, Dir}`. These types provide sandboxed access
//! to the local filesystem on both Unix and Windows.
//!
//! All syscalls are hidden behind the `cap-std` hierarchy, with the lone
//! exception of the `sched` implementation, which is provided for both unix
//! and windows in separate modules.
//!
//! Any `wasi_common::{WasiCtx, WasiCtxBuilder}` is interoperable with the
//! implementations provided in `wasi_common::sync`. However, for convenience,
//! this module provides its own `WasiCtxBuilder` that hooks up to all of the
//! crate's components, i.e. it fills in all of the arguments to
//! `WasiCtx::builder(...)`, presents `preopen_dir` in terms of
//! `cap_std::fs::Dir`, and provides convenience methods for inheriting the
//! parent process's stdio, args, and env.

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

use self::net::Socket;
use crate::{file::FileAccessMode, table::Table, Error, WasiCtx, WasiFile};
use cap_rand::{Rng, RngCore, SeedableRng};
use std::mem;
use std::path::Path;

pub struct WasiCtxBuilder {
    ctx: WasiCtx,
    built: bool,
}

impl WasiCtxBuilder {
    pub fn new() -> Self {
        WasiCtxBuilder {
            ctx: WasiCtx::new(random_ctx(), clocks_ctx(), sched_ctx(), Table::new()),
            built: false,
        }
    }
    pub fn env(&mut self, var: &str, value: &str) -> Result<&mut Self, crate::StringArrayError> {
        self.ctx.push_env(var, value)?;
        Ok(self)
    }
    pub fn envs(&mut self, env: &[(String, String)]) -> Result<&mut Self, crate::StringArrayError> {
        for (k, v) in env {
            self.ctx.push_env(k, v)?;
        }
        Ok(self)
    }
    pub fn inherit_env(&mut self) -> Result<&mut Self, crate::StringArrayError> {
        for (key, value) in std::env::vars() {
            self.ctx.push_env(&key, &value)?;
        }
        Ok(self)
    }
    pub fn arg(&mut self, arg: &str) -> Result<&mut Self, crate::StringArrayError> {
        self.ctx.push_arg(arg)?;
        Ok(self)
    }
    pub fn args(&mut self, arg: &[String]) -> Result<&mut Self, crate::StringArrayError> {
        for a in arg {
            self.ctx.push_arg(&a)?;
        }
        Ok(self)
    }
    pub fn inherit_args(&mut self) -> Result<&mut Self, crate::StringArrayError> {
        for arg in std::env::args() {
            self.ctx.push_arg(&arg)?;
        }
        Ok(self)
    }
    pub fn stdin(&mut self, f: Box<dyn WasiFile>) -> &mut Self {
        self.ctx.set_stdin(f);
        self
    }
    pub fn stdout(&mut self, f: Box<dyn WasiFile>) -> &mut Self {
        self.ctx.set_stdout(f);
        self
    }
    pub fn stderr(&mut self, f: Box<dyn WasiFile>) -> &mut Self {
        self.ctx.set_stderr(f);
        self
    }
    pub fn inherit_stdin(&mut self) -> &mut Self {
        self.stdin(Box::new(crate::sync::stdio::stdin()))
    }
    pub fn inherit_stdout(&mut self) -> &mut Self {
        self.stdout(Box::new(crate::sync::stdio::stdout()))
    }
    pub fn inherit_stderr(&mut self) -> &mut Self {
        self.stderr(Box::new(crate::sync::stdio::stderr()))
    }
    pub fn inherit_stdio(&mut self) -> &mut Self {
        self.inherit_stdin().inherit_stdout().inherit_stderr()
    }
    pub fn preopened_dir(
        &mut self,
        dir: Dir,
        guest_path: impl AsRef<Path>,
    ) -> Result<&mut Self, Error> {
        let dir = Box::new(crate::sync::dir::Dir::from_cap_std(dir));
        self.ctx.push_preopened_dir(dir, guest_path)?;
        Ok(self)
    }
    pub fn preopened_socket(
        &mut self,
        fd: u32,
        socket: impl Into<Socket>,
    ) -> Result<&mut Self, Error> {
        let socket: Socket = socket.into();
        let file: Box<dyn WasiFile> = socket.into();
        self.ctx
            .insert_file(fd, file, FileAccessMode::READ | FileAccessMode::WRITE);
        Ok(self)
    }
    pub fn build(&mut self) -> WasiCtx {
        assert!(!self.built);
        let WasiCtxBuilder { ctx, .. } = mem::replace(self, Self::new());
        self.built = true;
        ctx
    }
}

pub fn random_ctx() -> Box<dyn RngCore + Send + Sync> {
    let mut rng = cap_rand::thread_rng(cap_rand::ambient_authority());
    Box::new(cap_rand::rngs::StdRng::from_seed(rng.r#gen()))
}

#[cfg(feature = "wasmtime")]
super::define_wasi!(block_on);
