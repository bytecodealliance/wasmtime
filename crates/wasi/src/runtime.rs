//! This module provides an "ambient Tokio runtime"
//! [`with_ambient_tokio_runtime`]. Embedders of wasmtime-wasi may do so from
//! synchronous Rust, and not use tokio directly. The implementation of
//! wasmtime-wasi requires a tokio executor in a way that is [deeply tied to
//! its
//! design](https://github.com/bytecodealliance/wasmtime/issues/7973#issuecomment-1960513214).
//! When used from a sychrnonous wasmtime context, this module provides the
//! wrapper function [`in_tokio`] used throughout the shim implementations of
//! synchronous component binding `Host` traits in terms of the async ones.
//!
//! This module also provides a thin wrapper on tokio's tasks.
//! [`AbortOnDropJoinHandle`], which is exactly like a
//! [`tokio::task::JoinHandle`] except for the obvious behavioral change. This
//! whole crate, and any child crates which spawn tasks as part of their
//! implementations, should please use this crate's [`spawn`] and
//! [`spawn_blocking`] over tokio's. so we wanted the type name to stick out
//! if someone misses it.
//!
//! Each of these facilities should be used by dependencies of wasmtime-wasi
//! which when implementing component bindings.

mod task;

use std::future::Future;
use std::pin::Pin;
use std::sync::LazyLock;
use std::task::{Context, Poll};

pub use task::AbortOnDropJoinHandle;

pub trait WasiExecutor: Send + Sync + 'static {
    /// Configures whether or not blocking operations made through this
    /// `WasiExecutor` are allowed to block the current thread.
    ///
    /// Both `WasiExecutor` impls are is currently built on top of the Rust
    /// [Tokio](https://tokio.rs/) library. While most WASI APIs are
    /// non-blocking, some are instead blocking from the perspective of
    /// WebAssembly. For example opening a file is a blocking operation with
    /// respect to WebAssembly but it's implemented as an asynchronous operation
    /// on the host. This is currently done with Tokio's
    /// [`spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).
    ///
    /// When WebAssembly is used in a synchronous context, for example when
    /// [`Config::async_support`] is disabled, then this asynchronous operation
    /// is quickly turned back into a synchronous operation with a `block_on` in
    /// Rust (specifically the `WasiSyncExecutor::block_on`, which is a proxy
    /// for tokio's in the provided `Standalone` impl). This switching
    /// back-and-forth between a blocking a non-blocking context can have
    /// overhead, and this option exists to help alleviate this overhead.
    ///
    /// This option indicates that for WASI functions that are blocking from the
    /// perspective of WebAssembly it's ok to block the native thread as well.
    /// This means that this back-and-forth between async and sync won't happen
    /// and instead blocking operations are performed on-thread (such as opening
    /// a file). This can improve the performance of WASI operations when async
    /// support is disabled.
    ///
    /// [`Config::async_support`]: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.async_support
    fn run_blocking<F, R>(body: F) -> impl std::future::Future<Output = R> + Send
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static;

    fn spawn<F>(f: F) -> AbortOnDropJoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;

    fn spawn_blocking<F, R>(f: F) -> AbortOnDropJoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static;
}

pub trait WasiSyncExecutor: WasiExecutor {
    fn block_on<F>(f: F) -> F::Output
    where
        F: Future;
}

pub struct Tokio;
impl WasiExecutor for Tokio {
    async fn run_blocking<F, R>(body: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        Self::spawn_blocking(move || body()).await
    }

    fn spawn<F>(f: F) -> AbortOnDropJoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let j = tokio::task::spawn(f);
        AbortOnDropJoinHandle::from(j)
    }

    fn spawn_blocking<F, R>(f: F) -> AbortOnDropJoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let j = tokio::task::spawn_blocking(f);
        AbortOnDropJoinHandle::from(j)
    }
}
pub struct Standalone;
impl WasiExecutor for Standalone {
    /// Execute the blocking `body` function.
    ///
    /// This implementation runs the blocking `body` directly on the current
    /// thread. In this case the `async` signature of this method is
    /// effectively a lie and the returned Future will always be immediately
    /// Ready.
    ///
    /// Intentionally blocking the executor thread might seem unorthodox, but
    /// is not actually a problem for specific workloads. See:
    /// - [`crate::WasiExecutor::run_blocking`]
    /// - [Poor performance of wasmtime file I/O maybe because tokio](https://github.com/bytecodealliance/wasmtime/issues/7973)
    /// - [Implement opt-in for enabling WASI to block the current thread](https://github.com/bytecodealliance/wasmtime/pull/8190)
    async fn run_blocking<F, R>(body: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        body()
    }

    fn spawn<F>(f: F) -> AbortOnDropJoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let j = with_ambient_tokio_runtime(|| tokio::task::spawn(f));
        AbortOnDropJoinHandle::from(j)
    }

    fn spawn_blocking<F, R>(f: F) -> AbortOnDropJoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let j = with_ambient_tokio_runtime(|| tokio::task::spawn_blocking(f));
        AbortOnDropJoinHandle::from(j)
    }
}
impl WasiSyncExecutor for Standalone {
    fn block_on<F>(f: F) -> F::Output
    where
        F: Future,
    {
        in_tokio(f)
    }
}

pub(crate) static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap()
});

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
