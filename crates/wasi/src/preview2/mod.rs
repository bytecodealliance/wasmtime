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

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

mod clocks;
pub mod command;
mod ctx;
mod error;
mod filesystem;
mod host;
mod ip_name_lookup;
mod network;
pub mod pipe;
mod poll;
#[cfg(feature = "preview1-on-preview2")]
pub mod preview0;
#[cfg(feature = "preview1-on-preview2")]
pub mod preview1;
mod random;
mod stdio;
mod stream;
mod tcp;
mod udp;
mod write_stream;

pub use self::clocks::{HostMonotonicClock, HostWallClock};
pub use self::ctx::{WasiCtx, WasiCtxBuilder, WasiView};
pub use self::error::{I32Exit, TrappableError};
pub use self::filesystem::{DirPerms, FilePerms, FsError, FsResult};
pub use self::network::{Network, SocketError, SocketResult};
pub use self::poll::{subscribe, ClosureFuture, MakeFuture, Pollable, PollableFuture, Subscribe};
pub use self::random::{thread_rng, Deterministic};
pub use self::stdio::{
    stderr, stdin, stdout, IsATTY, Stderr, Stdin, StdinStream, Stdout, StdoutStream,
};
pub use self::stream::{
    HostInputStream, HostOutputStream, InputStream, OutputStream, StreamError, StreamResult,
};
pub use cap_fs_ext::SystemTimeSpec;
pub use cap_rand::RngCore;
pub use wasmtime::component::{ResourceTable, ResourceTableError};

pub mod bindings {
    // Generate traits for synchronous bindings.
    //
    // Note that this is only done for interfaces which can block, or those which
    // have some functions in `only_imports` below for being async.
    pub mod sync_io {
        pub(crate) mod _internal {
            use crate::preview2::{FsError, StreamError};

            wasmtime::component::bindgen!({
                path: "wit",
                interfaces: "
                    import wasi:io/poll@0.2.0-rc-2023-11-10;
                    import wasi:io/streams@0.2.0-rc-2023-11-10;
                    import wasi:filesystem/types@0.2.0-rc-2023-11-10;
                ",
                tracing: true,
                trappable_error_type: {
                    "wasi:io/streams/stream-error" => StreamError,
                    "wasi:filesystem/types/error-code" => FsError,
                },
                with: {
                    "wasi:clocks/wall-clock": crate::preview2::bindings::clocks::wall_clock,
                    "wasi:filesystem/types/descriptor": super::super::filesystem::types::Descriptor,
                    "wasi:filesystem/types/directory-entry-stream": super::super::filesystem::types::DirectoryEntryStream,
                    "wasi:io/poll/pollable": super::super::io::poll::Pollable,
                    "wasi:io/streams/input-stream": super::super::io::streams::InputStream,
                    "wasi:io/streams/output-stream": super::super::io::streams::OutputStream,
                    "wasi:io/error/error": super::super::io::error::Error,
                }
            });
        }
        pub use self::_internal::wasi::{filesystem, io};
    }

    wasmtime::component::bindgen!({
        path: "wit",
        world: "wasi:cli/imports",
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
                "poll",
                "[method]pollable.block",
                "[method]pollable.ready",
            ],
        },
        trappable_error_type: {
            "wasi:io/streams/stream-error" => crate::preview2::StreamError,
            "wasi:filesystem/types/error-code" => crate::preview2::FsError,
            "wasi:sockets/network/error-code" => crate::preview2::SocketError,
        },
        with: {
            "wasi:sockets/network/network": super::network::Network,
            "wasi:sockets/tcp/tcp-socket": super::tcp::TcpSocket,
            "wasi:sockets/udp/udp-socket": super::udp::UdpSocket,
            "wasi:sockets/udp/incoming-datagram-stream": super::udp::IncomingDatagramStream,
            "wasi:sockets/udp/outgoing-datagram-stream": super::udp::OutgoingDatagramStream,
            "wasi:sockets/ip-name-lookup/resolve-address-stream": super::ip_name_lookup::ResolveAddressStream,
            "wasi:filesystem/types/directory-entry-stream": super::filesystem::ReaddirIterator,
            "wasi:filesystem/types/descriptor": super::filesystem::Descriptor,
            "wasi:io/streams/input-stream": super::stream::InputStream,
            "wasi:io/streams/output-stream": super::stream::OutputStream,
            "wasi:io/error/error": super::stream::Error,
            "wasi:io/poll/pollable": super::poll::Pollable,
            "wasi:cli/terminal-input/terminal-input": super::stdio::TerminalInput,
            "wasi:cli/terminal-output/terminal-output": super::stdio::TerminalOutput,
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
impl<T> Future for AbortOnDropJoinHandle<T> {
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.as_mut().0).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(r) => Poll::Ready(r.expect("child task panicked")),
        }
    }
}

pub fn spawn<F>(f: F) -> AbortOnDropJoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let j = with_ambient_tokio_runtime(|| tokio::task::spawn(f));
    AbortOnDropJoinHandle(j)
}

pub fn spawn_blocking<F, R>(f: F) -> AbortOnDropJoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let j = with_ambient_tokio_runtime(|| tokio::task::spawn_blocking(f));
    AbortOnDropJoinHandle(j)
}

pub fn in_tokio<F: Future>(f: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => {
            let _enter = h.enter();
            h.block_on(f)
        }
        // The `yield_now` here is non-obvious and if you're reading this
        // you're likely curious about why it's here. This is currently required
        // to get some features of "sync mode" working correctly, such as with
        // the CLI. To illustrate why this is required, consider a program
        // organized as:
        //
        // * A program has a `pollable` that it's waiting on.
        // * This `pollable` is always ready .
        // * Actually making the corresponding operation ready, however,
        //   requires some background work on Tokio's part.
        // * The program is looping on "wait for readiness" coupled with
        //   performing the operation.
        //
        // In this situation this program ends up infinitely looping in waiting
        // for pollables. The reason appears to be that when we enter the tokio
        // runtime here it doesn't necessary yield to background work because
        // the provided future `f` is ready immediately. The future `f` will run
        // through the list of pollables and determine one of them is ready.
        //
        // Historically this happened with UDP sockets. A test send a datagram
        // from one socket to another and the other socket infinitely didn't
        // receive the data. This appeared to be because the server socket was
        // waiting on `READABLE | WRITABLE` (which is itself a bug but ignore
        // that) and the socket was currently in the "writable" state but never
        // ended up receiving a notification for the "readable" state. Moving
        // the socket to "readable" would require Tokio to perform some
        // background work via epoll/kqueue/handle events but if the future
        // provided here is always ready, then that never happened.
        //
        // Thus the `yield_now()` is an attempt to force Tokio to go do some
        // background work eventually and look at new interest masks for
        // example. This is a bit of a kludge but everything's already a bit
        // wonky in synchronous mode anyway. Note that this is hypothesized to
        // not be an issue in async mode because async mode typically has the
        // Tokio runtime in a separate thread or otherwise participating in a
        // larger application, it's only here in synchronous mode where we
        // effectively own the runtime that we need some special care.
        Err(_) => {
            let _enter = RUNTIME.enter();
            RUNTIME.block_on(async move {
                tokio::task::yield_now().await;
                f.await
            })
        }
    }
}

/// Executes the closure `f` with an "ambient Tokio runtime" which basically
/// means that if code in `f` tries to get a runtime `Handle` it'll succeed.
///
/// If a `Handle` is already available, e.g. in async contexts, then `f` is run
/// immediately. Otherwise for synchronous contexts this crate's fallback
/// runtime is configured and then `f` is executed.
pub fn with_ambient_tokio_runtime<R>(f: impl FnOnce() -> R) -> R {
    match tokio::runtime::Handle::try_current() {
        Ok(_) => f(),
        Err(_) => {
            let _enter = RUNTIME.enter();
            f()
        }
    }
}

/// Attempts to get the result of a `future`.
///
/// This function does not block and will poll the provided future once. If the
/// result is here then `Some` is returned, otherwise `None` is returned.
///
/// Note that by polling `future` this means that `future` must be re-polled
/// later if it's to wake up a task.
pub fn poll_noop<F>(future: Pin<&mut F>) -> Option<F::Output>
where
    F: Future,
{
    let mut task = Context::from_waker(futures::task::noop_waker_ref());
    match future.poll(&mut task) {
        Poll::Ready(result) => Some(result),
        Poll::Pending => None,
    }
}
