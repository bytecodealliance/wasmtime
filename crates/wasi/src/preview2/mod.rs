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
mod network;
pub mod pipe;
mod poll;
#[cfg(feature = "preview1-on-preview2")]
pub mod preview1;
mod random;
mod stdio;
mod stream;
mod table;
mod tcp;
mod write_stream;

pub use self::clocks::{HostMonotonicClock, HostWallClock};
pub use self::ctx::{WasiCtx, WasiCtxBuilder, WasiView};
pub use self::error::I32Exit;
pub use self::filesystem::{DirPerms, FilePerms};
pub use self::poll::{ClosureFuture, HostPollable, MakeFuture, PollableFuture, TablePollableExt};
pub use self::random::{thread_rng, Deterministic};
pub use self::stdio::{stderr, stdin, stdout, IsATTY, Stderr, Stdin, Stdout};
pub use self::stream::{
    HostInputStream, HostOutputStream, OutputStreamError, StreamRuntimeError, StreamState,
    TableStreamExt,
};
pub use self::table::{OccupiedEntry, Table, TableError};
pub use cap_fs_ext::SystemTimeSpec;
pub use cap_rand::RngCore;

pub mod bindings {
    // Generate traits for synchronous bindings.
    //
    // Note that this is only done for interfaces which can block, or those which
    // have some functions in `only_imports` below for being async.
    pub mod sync_io {
        pub(crate) mod _internal {
            wasmtime::component::bindgen!({
                path: "wit",
                interfaces: "
                    import wasi:io/poll
                    import wasi:io/streams
                    import wasi:filesystem/types
                ",
                tracing: true,
                trappable_error_type: {
                    "wasi:io/streams"::"write-error": Error,
                    "wasi:filesystem/types"::"error-code": Error,
                },
                with: {
                    "wasi:clocks/wall-clock": crate::preview2::bindings::clocks::wall_clock,
                }
            });
        }
        pub use self::_internal::wasi::{filesystem, io};
    }

    wasmtime::component::bindgen!({
        path: "wit",
        interfaces: "include wasi:cli/reactor",
        tracing: true,
        async: {
            // Only these functions are `async` and everything else is sync
            // meaning that it basically doesn't need to block. These functions
            // are the only ones that need to block.
            //
            // Note that at this time `only_imports` works on function names
            // which in theory can be shared across interfaces, so this may
            // need fancier syntax in the future.
            only_imports: [
                "[method]descriptor.access-at",
                "[method]descriptor.advise",
                "[method]descriptor.change-directory-permissions-at",
                "[method]descriptor.change-file-permissions-at",
                "[method]descriptor.create-directory-at",
                "[method]descriptor.get-flags",
                "[method]descriptor.get-type",
                "[method]descriptor.is-same-object",
                "[method]descriptor.link-at",
                "[method]descriptor.lock-exclusive",
                "[method]descriptor.lock-shared",
                "[method]descriptor.metadata-hash",
                "[method]descriptor.metadata-hash-at",
                "[method]descriptor.open-at",
                "[method]descriptor.read",
                "[method]descriptor.read-directory",
                "[method]descriptor.readlink-at",
                "[method]descriptor.remove-directory-at",
                "[method]descriptor.rename-at",
                "[method]descriptor.set-size",
                "[method]descriptor.set-times",
                "[method]descriptor.set-times-at",
                "[method]descriptor.stat",
                "[method]descriptor.stat-at",
                "[method]descriptor.symlink-at",
                "[method]descriptor.sync",
                "[method]descriptor.sync-data",
                "[method]descriptor.try-lock-exclusive",
                "[method]descriptor.try-lock-shared",
                "[method]descriptor.unlink-file-at",
                "[method]descriptor.unlock",
                "[method]descriptor.write",
                "[method]input-stream.read",
                "[method]input-stream.blocking-read",
                "[method]input-stream.blocking-skip",
                "[method]input-stream.skip",
                "[method]output-stream.forward",
                "[method]output-stream.splice",
                "[method]output-stream.blocking-splice",
                "[method]output-stream.blocking-flush",
                "[method]output-stream.blocking-write",
                "[method]output-stream.blocking-write-and-flush",
                "[method]output-stream.blocking-write-zeroes-and-flush",
                "[method]directory-entry-stream.read-directory-entry",
                "poll-list",
                "poll-one",
            ],
        },
        trappable_error_type: {
            "wasi:io/streams"::"write-error": Error,
            "wasi:filesystem/types"::"error-code": Error,
            "wasi:sockets/network"::"error-code": Error,
        },
    });

    pub use wasi::*;
}

pub(crate) static RUNTIME: once_cell::sync::Lazy<tokio::runtime::Runtime> =
    once_cell::sync::Lazy::new(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .unwrap()
    });

pub struct AbortOnDropJoinHandle<T>(tokio::task::JoinHandle<T>);
impl<T> Drop for AbortOnDropJoinHandle<T> {
    fn drop(&mut self) {
        self.0.abort()
    }
}
impl<T> std::ops::Deref for AbortOnDropJoinHandle<T> {
    type Target = tokio::task::JoinHandle<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> std::ops::DerefMut for AbortOnDropJoinHandle<T> {
    fn deref_mut(&mut self) -> &mut tokio::task::JoinHandle<T> {
        &mut self.0
    }
}
impl<T> From<tokio::task::JoinHandle<T>> for AbortOnDropJoinHandle<T> {
    fn from(jh: tokio::task::JoinHandle<T>) -> Self {
        AbortOnDropJoinHandle(jh)
    }
}
impl<T> std::future::Future for AbortOnDropJoinHandle<T> {
    type Output = T;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        use std::pin::Pin;
        use std::task::Poll;
        match Pin::new(&mut self.as_mut().0).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(r) => Poll::Ready(r.expect("child task panicked")),
        }
    }
}

pub fn spawn<F, G>(f: F) -> AbortOnDropJoinHandle<G>
where
    F: std::future::Future<Output = G> + Send + 'static,
    G: Send + 'static,
{
    let j = match tokio::runtime::Handle::try_current() {
        Ok(_) => tokio::task::spawn(f),
        Err(_) => {
            let _enter = RUNTIME.enter();
            tokio::task::spawn(f)
        }
    };
    AbortOnDropJoinHandle(j)
}

pub fn in_tokio<F: std::future::Future>(f: F) -> F::Output {
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

fn with_ambient_tokio_runtime<R>(f: impl FnOnce() -> R) -> R {
    match tokio::runtime::Handle::try_current() {
        Ok(_) => f(),
        Err(_) => {
            let _enter = RUNTIME.enter();
            f()
        }
    }
}
