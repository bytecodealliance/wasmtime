//! The `wasi-cap-std-sync` crate provides impl of `WasiFile` and `WasiDir` in
//! terms of `cap_std::fs::{File, Dir}`. These types provide sandboxed access
//! to the local filesystem on both Unix and Windows.
//!
//! All syscalls are hidden behind the `cap-std` hierarchy, with the lone
//! exception of the `sched` implementation.
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
pub mod sched;
pub mod stdio;

pub use cap_std::fs::Dir;
pub use cap_std::AmbientAuthority;
pub use clocks::clocks_ctx;

use cap_rand::{Rng, RngCore, SeedableRng};
use wasi_common::{
    pipe::{ReadPipe, WritePipe},
    stream::{InputStream, OutputStream},
    table::Table,
    WasiCtx, WasiCtxBuilder as B,
};

pub struct WasiCtxBuilder(B);

impl WasiCtxBuilder {
    pub fn new() -> Self {
        WasiCtxBuilder(
            B::default()
                .set_random(random_ctx())
                .set_clocks(clocks_ctx())
                .set_sched(sched::SyncSched)
                .set_stdin(ReadPipe::new(std::io::empty()))
                .set_stdout(WritePipe::new(std::io::sink()))
                .set_stderr(WritePipe::new(std::io::sink())),
        )
    }
    pub fn stdin(mut self, f: impl InputStream + 'static) -> Self {
        self.0 = self.0.set_stdin(f);
        self
    }
    pub fn stdout(mut self, f: impl OutputStream + 'static) -> Self {
        self.0 = self.0.set_stdout(f);
        self
    }
    pub fn stderr(mut self, f: impl OutputStream + 'static) -> Self {
        self.0 = self.0.set_stderr(f);
        self
    }
    pub fn set_args(mut self, args: &[impl AsRef<str>]) -> Self {
        self.0 = self.0.set_args(args);
        self
    }
    pub fn push_env(mut self, k: impl AsRef<str>, v: impl AsRef<str>) -> Self {
        self.0 = self.0.push_env(k, v);
        self
    }
    pub fn inherit_stdin(self) -> Self {
        self.stdin(crate::stdio::stdin())
    }
    pub fn inherit_stdout(self) -> Self {
        self.stdout(crate::stdio::stdout())
    }
    pub fn inherit_stderr(self) -> Self {
        self.stderr(crate::stdio::stderr())
    }
    pub fn inherit_stdio(self) -> Self {
        self.inherit_stdin().inherit_stdout().inherit_stderr()
    }
    pub fn preopened_dir(mut self, dir: cap_std::fs::Dir, guest_path: &str) -> Self {
        let dir = crate::dir::Dir::from_cap_std(dir);
        self.0 = self.0.push_preopened_dir(dir, guest_path);
        self
    }
    pub fn preopened_dir_impl(
        mut self,
        dir: impl wasi_common::WasiDir + 'static,
        guest_path: &str,
    ) -> Self {
        self.0 = self.0.push_preopened_dir(dir, guest_path);
        self
    }
    pub fn args(mut self, args: &[impl AsRef<str>]) -> Self {
        self.0 = self.0.set_args(args);
        self
    }
    pub fn build(self, table: &mut Table) -> anyhow::Result<WasiCtx> {
        self.0.build(table)
    }
}

pub fn random_ctx() -> Box<dyn RngCore + Send + Sync> {
    let mut rng = cap_rand::thread_rng(cap_rand::ambient_authority());
    Box::new(cap_rand::rngs::StdRng::from_seed(rng.gen()))
}
