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
pub mod pipe;
mod poll;
#[cfg(feature = "preview1-on-preview2")]
pub mod preview1;
mod preview2;
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
pub use self::stream::{HostInputStream, HostOutputStream, StreamState, TableStreamExt};
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
              import wasi:filesystem/filesystem
            ",
                tracing: true,
                trappable_error_type: {
                    "wasi:io/streams"::"stream-error": Error,
                    "wasi:filesystem/filesystem"::"error-code": Error,
                },
                with: {
                    "wasi:clocks/wall-clock": crate::preview2::bindings::clocks::wall_clock,
                }
            });
        }
        pub use self::_internal::wasi::{filesystem, io, poll};

        impl From<super::io::streams::StreamError> for io::streams::StreamError {
            fn from(other: super::io::streams::StreamError) -> Self {
                let super::io::streams::StreamError { dummy } = other;
                Self { dummy }
            }
        }

        impl From<super::io::streams::Error> for io::streams::Error {
            fn from(other: super::io::streams::Error) -> Self {
                match other.downcast() {
                    Ok(se) => io::streams::Error::from(io::streams::StreamError::from(se)),
                    Err(e) => io::streams::Error::trap(e),
                }
            }
        }
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
              import wasi:filesystem/filesystem
            ",
            tracing: true,
            async: true,
            trappable_error_type: {
                "wasi:io/streams"::"stream-error": Error,
                "wasi:filesystem/filesystem"::"error-code": Error,
            },
            with: {
                "wasi:clocks/wall-clock": crate::preview2::bindings::clocks::wall_clock,
            }
        });
    }
    pub use self::_internal_io::wasi::{filesystem, io, poll};

    pub(crate) mod _internal_rest {
        wasmtime::component::bindgen!({
        path: "wit",
        interfaces: "
              import wasi:clocks/wall-clock
              import wasi:clocks/monotonic-clock
              import wasi:clocks/timezone
              import wasi:random/random
              import wasi:random/insecure
              import wasi:random/insecure-seed
              import wasi:cli-base/environment
              import wasi:cli-base/preopens
              import wasi:cli-base/exit
              import wasi:cli-base/stdin
              import wasi:cli-base/stdout
              import wasi:cli-base/stderr
            ",
        tracing: true,
        trappable_error_type: {
            "wasi:filesystem/filesystem"::"error-code": Error,
            "wasi:io/streams"::"stream-error": Error,
        },
        with: {
            "wasi:clocks/wall-clock": crate::preview2::bindings::clocks::wall_clock,
            "wasi:poll/poll": crate::preview2::bindings::poll::poll,
            "wasi:io/streams": crate::preview2::bindings::io::streams,
            "wasi:filesystem/filesystem": crate::preview2::bindings::filesystem::filesystem
        }
        });
    }

    pub use self::_internal_rest::wasi::*;
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
