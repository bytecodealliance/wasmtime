//! # Wasmtime's WASI Preview 2 Implementation
//!
//! Welcome to the (new!) WASI implementation from the Wasmtime team. The goal
//! of this implementation is to support WASI Preview 2 via the Component
//! Model, as well as to provide legacy Preview 1 host support with an adapter
//! that is implemented in terms of the Preview 2 interfaces.
//!
//! Presently, this crate is experimental. We don't yet recommend you use it
//! in production. Specifically:
//! * the wit files in tree describing preview 2 are not faithful to the
//! standards repos
//!
//! Once these issues are resolved, we expect to move this namespace up to the
//! root of the wasmtime-wasi crate, and move its other exports underneath a
//! `pub mod legacy` with an off-by-default feature flag, and after 2
//! releases, retire and remove that code from our tree.

mod clocks;
pub mod command;
mod ctx;
mod error;
mod filesystem;
mod host;
pub mod pipe;
mod poll;
#[cfg(feature = "preview1-on-preview2")]
pub mod preview1;
mod random;
mod stdio;
mod stream;
mod table;

pub use self::clocks::{HostMonotonicClock, HostWallClock};
pub use self::ctx::{WasiCtx, WasiCtxBuilder, WasiView};
pub use self::error::I32Exit;
pub use self::filesystem::{DirPerms, FilePerms};
pub use self::poll::{ClosureFuture, HostPollable, MakeFuture, PollableFuture, TablePollableExt};
pub use self::random::{thread_rng, Deterministic};
pub use self::stdio::{stderr, stdin, stdout, IsATTY, Stderr, Stdin, Stdout};
pub use self::stream::{
    HostInputStream, HostOutputStream, StreamRuntimeError, StreamState, TableStreamExt,
};
pub use self::table::{OccupiedEntry, Table, TableError};
pub use cap_fs_ext::SystemTimeSpec;
pub use cap_rand::RngCore;

pub mod bindings {
    pub mod sync_io {
        pub(crate) mod _internal {
            wasmtime::component::bindgen!({
                path: "wit",
                interfaces: "
              import wasi:poll/poll
              import wasi:io/streams
              import wasi:filesystem/types
            ",
                tracing: true,
                trappable_error_type: {
                    "wasi:filesystem/types"::"error-code": Error,
                },
                with: {
                    "wasi:clocks/wall-clock": crate::preview2::bindings::clocks::wall_clock,
                }
            });
        }
        pub use self::_internal::wasi::{filesystem, io, poll};
    }

    pub(crate) mod _internal_clocks {
        wasmtime::component::bindgen!({
        path: "wit",
        interfaces: "
              import wasi:clocks/wall-clock
              import wasi:clocks/monotonic-clock
              import wasi:clocks/timezone
            ",
        tracing: true,
        });
    }
    pub use self::_internal_clocks::wasi::clocks;

    pub(crate) mod _internal_io {
        wasmtime::component::bindgen!({
            path: "wit",
            interfaces: "
              import wasi:poll/poll
              import wasi:io/streams
              import wasi:filesystem/types
            ",
            tracing: true,
            async: true,
            trappable_error_type: {
                "wasi:filesystem/types"::"error-code": Error,
            },
            with: {
                "wasi:clocks/wall-clock": crate::preview2::bindings::clocks::wall_clock,
            }
        });
    }
    pub use self::_internal_io::wasi::{io, poll};

    pub(crate) mod _internal_rest {
        wasmtime::component::bindgen!({
        path: "wit",
        interfaces: "
              import wasi:filesystem/preopens
              import wasi:random/random
              import wasi:random/insecure
              import wasi:random/insecure-seed
              import wasi:cli/environment
              import wasi:cli/exit
              import wasi:cli/stdin
              import wasi:cli/stdout
              import wasi:cli/stderr
              import wasi:cli/terminal-input
              import wasi:cli/terminal-output
              import wasi:cli/terminal-stdin
              import wasi:cli/terminal-stdout
              import wasi:cli/terminal-stderr
            ",
        tracing: true,
        trappable_error_type: {
            "wasi:filesystem/types"::"error-code": Error,
        },
        with: {
            "wasi:clocks/wall-clock": crate::preview2::bindings::clocks::wall_clock,
            "wasi:poll/poll": crate::preview2::bindings::poll::poll,
            "wasi:io/streams": crate::preview2::bindings::io::streams,
            "wasi:filesystem/types": crate::preview2::bindings::filesystem::types,
        }
        });
    }

    pub use self::_internal_rest::wasi::{cli, random};
    pub mod filesystem {
        pub use super::_internal_io::wasi::filesystem::types;
        pub use super::_internal_rest::wasi::filesystem::preopens;
    }
}

pub(crate) static RUNTIME: once_cell::sync::Lazy<tokio::runtime::Runtime> =
    once_cell::sync::Lazy::new(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .unwrap()
    });

pub(crate) fn spawn<F, G>(f: F) -> tokio::task::JoinHandle<G>
where
    F: std::future::Future<Output = G> + Send + 'static,
    G: Send + 'static,
{
    match tokio::runtime::Handle::try_current() {
        Ok(_) => tokio::task::spawn(f),
        Err(_) => {
            let _enter = RUNTIME.enter();
            tokio::task::spawn(f)
        }
    }
}

pub(crate) fn in_tokio<F: std::future::Future>(f: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => {
            let _enter = h.enter();
            h.block_on(f)
        }
        Err(_) => {
            let _enter = RUNTIME.enter();
            RUNTIME.block_on(f)
        }
    }
}
