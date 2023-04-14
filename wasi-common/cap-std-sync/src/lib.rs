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
pub mod net;
pub mod sched;
pub mod stdio;

pub use cap_std::ambient_authority;
pub use cap_std::fs::Dir;
pub use cap_std::net::TcpListener;
pub use clocks::clocks_ctx;
pub use sched::sched_ctx;

use crate::net::{Network, TcpSocket};
use cap_net_ext::AddressFamily;
use cap_rand::{Rng, RngCore, SeedableRng};
use cap_std::net::{Ipv4Addr, Ipv6Addr, Pool};
use ipnet::IpNet;
use wasi_common::{
    network::WasiNetwork,
    stream::{InputStream, OutputStream},
    table::Table,
    tcp_socket::WasiTcpSocket,
    Error, WasiCtx,
};

pub struct WasiCtxBuilder(WasiCtx);

impl WasiCtxBuilder {
    pub fn new() -> Self {
        WasiCtxBuilder(WasiCtx::new(
            random_ctx(),
            clocks_ctx(),
            sched_ctx(),
            Table::new(),
            Box::new(create_network),
            Box::new(create_tcp_socket),
        ))
    }
    pub fn stdin(mut self, f: Box<dyn InputStream>) -> Self {
        self.0.set_stdin(f);
        self
    }
    pub fn stdout(mut self, f: Box<dyn OutputStream>) -> Self {
        self.0.set_stdout(f);
        self
    }
    pub fn stderr(mut self, f: Box<dyn OutputStream>) -> Self {
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
    pub fn inherit_network(mut self) -> Self {
        self.0
            .insert_ip_net_port_any(IpNet::new(Ipv4Addr::UNSPECIFIED.into(), 0).unwrap());
        self.0
            .insert_ip_net_port_any(IpNet::new(Ipv6Addr::UNSPECIFIED.into(), 0).unwrap());
        self
    }
    pub fn preopened_dir(mut self, fd: u32, dir: Dir) -> Self {
        let dir = Box::new(crate::dir::Dir::from_cap_std(dir));
        self.0.insert_dir(fd, dir);
        self
    }
    pub fn preopened_listener(mut self, fd: u32, listener: impl Into<TcpSocket>) -> Self {
        let listener: TcpSocket = listener.into();
        let listener: Box<dyn WasiTcpSocket> = Box::new(TcpSocket::from(listener));

        self.0.insert_listener(fd, listener);
        self
    }
    pub fn args(mut self, args: &[impl AsRef<str>]) -> Self {
        self.0.set_args(args);
        self
    }
    pub fn build(self) -> WasiCtx {
        self.0
    }
}

fn create_network(pool: Pool) -> Result<Box<dyn WasiNetwork>, Error> {
    let network: Box<dyn WasiNetwork> = Box::new(Network::new(pool));
    Ok(network)
}

fn create_tcp_socket(address_family: AddressFamily) -> Result<Box<dyn WasiTcpSocket>, Error> {
    let socket: Box<dyn WasiTcpSocket> = Box::new(TcpSocket::new(address_family)?);
    Ok(socket)
}

pub fn random_ctx() -> Box<dyn RngCore + Send + Sync> {
    let mut rng = cap_rand::thread_rng(cap_rand::ambient_authority());
    Box::new(cap_rand::rngs::StdRng::from_seed(rng.gen()))
}
