#![cfg_attr(io_lifetimes_use_std, feature(io_safety))]

mod dir;
mod file;
pub mod net;
pub mod sched;
pub mod stdio;

use crate::sched::sched_ctx;
pub use dir::Dir;
pub use file::File;
pub use net::*;
use std::future::Future;
use std::mem;
use std::path::Path;
use wasi_cap_std_sync::net::Socket;
pub use wasi_cap_std_sync::{clocks_ctx, random_ctx};
use wasi_common::{file::FileAccessMode, Error, Table, WasiCtx, WasiFile};

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
    pub fn env(
        &mut self,
        var: &str,
        value: &str,
    ) -> Result<&mut Self, wasi_common::StringArrayError> {
        self.ctx.push_env(var, value)?;
        Ok(self)
    }
    pub fn envs(
        &mut self,
        env: &[(String, String)],
    ) -> Result<&mut Self, wasi_common::StringArrayError> {
        for (k, v) in env {
            self.ctx.push_env(k, v)?;
        }
        Ok(self)
    }
    pub fn inherit_env(&mut self) -> Result<&mut Self, wasi_common::StringArrayError> {
        for (key, value) in std::env::vars() {
            self.ctx.push_env(&key, &value)?;
        }
        Ok(self)
    }
    pub fn arg(&mut self, arg: &str) -> Result<&mut Self, wasi_common::StringArrayError> {
        self.ctx.push_arg(arg)?;
        Ok(self)
    }
    pub fn args(&mut self, arg: &[String]) -> Result<&mut Self, wasi_common::StringArrayError> {
        for a in arg {
            self.ctx.push_arg(&a)?;
        }
        Ok(self)
    }
    pub fn inherit_args(&mut self) -> Result<&mut Self, wasi_common::StringArrayError> {
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
        self.stdin(Box::new(crate::stdio::stdin()))
    }
    pub fn inherit_stdout(&mut self) -> &mut Self {
        self.stdout(Box::new(crate::stdio::stdout()))
    }
    pub fn inherit_stderr(&mut self) -> &mut Self {
        self.stderr(Box::new(crate::stdio::stderr()))
    }
    pub fn inherit_stdio(&mut self) -> &mut Self {
        self.inherit_stdin().inherit_stdout().inherit_stderr()
    }
    pub fn preopened_dir(
        &mut self,
        dir: cap_std::fs::Dir,
        guest_path: impl AsRef<Path>,
    ) -> Result<&mut Self, Error> {
        let dir = Box::new(crate::dir::Dir::from_cap_std(dir));
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
    tokio::task::block_in_place(move || {
        wiggle::run_in_dummy_executor(f()).expect("wrapped operation should be synchronous")
    })
}
